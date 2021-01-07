
use amq_protocol_uri::{AMQPAuthority, AMQPUserInfo};
use clap::{Arg, ArgMatches};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, QueueableCommand};
use futures_util::stream::StreamExt;
use lapin::options::BasicAckOptions;
use lapin::publisher_confirm::PublisherConfirm;
use lapin::{
  options::{BasicConsumeOptions, BasicPublishOptions},
  types::{AMQPValue, FieldTable},
  uri::{AMQPScheme, AMQPUri},
  BasicProperties, Connection, ConnectionProperties, ExchangeKind,
};
use mcai_worker_sdk::{
  debug, error,
  message_exchange::{
    rabbitmq::{
      channels::{BindDescription, ExchangeDescription, QueueDescription},
      EXCHANGE_NAME_DIRECT_MESSAGING,
      EXCHANGE_NAME_WORKER_RESPONSE,
      QUEUE_WORKER_CREATED,
      QUEUE_WORKER_INITIALIZED,
      QUEUE_WORKER_STARTED,
      QUEUE_WORKER_STATUS,
      QUEUE_WORKER_TERMINATED,
      QUEUE_WORKER_UPDATED,
      WORKER_RESPONSE_NOT_FOUND,
    },
    OrderMessage,
  },
  processor::ProcessStatus,
  Channel,
};
use std::collections::{BTreeMap, HashMap};
use std::io::{stdout, Write};
use std::process::exit;
use std::str::FromStr;
use std::sync::{
  mpsc::{self, Receiver, TryRecvError},
  Arc, Mutex,
};
use std::time::Duration;

pub fn get_request_headers(worker_id: Option<&str>) -> FieldTable {
  let mut headers = FieldTable::default();
  if let Some(worker) = worker_id {
    headers.insert(
      "worker_name".into(),
      AMQPValue::LongString(worker.to_string().into()),
    );
  } else {
    headers.insert("broadcast".into(), AMQPValue::Boolean(true));
  }
  headers
}

pub fn get_command_args() -> Vec<Arg<'static, 'static>> {
  vec![
    Arg::with_name("host")
      .short("h")
      .long("host")
      .takes_value(true)
      .default_value("127.0.0.1"),
    Arg::with_name("port")
      .short("p")
      .long("port")
      .takes_value(true)
      .default_value("5672"),
    Arg::with_name("virtual_host")
      .long("virtual-host")
      .takes_value(true)
      .default_value(""),
    Arg::with_name("user")
      .short("u")
      .long("user")
      .takes_value(true)
      .default_value("guest"),
    Arg::with_name("password")
      .short("P")
      .long("password")
      .takes_value(true)
      .default_value("guest"),
    Arg::with_name("tls").long("tls"),
    Arg::with_name("worker_id")
      .short("w")
      .long("worker-id")
      .takes_value(true),
  ]
}

pub fn status(matches: &ArgMatches) {
  match get_amqp_server_uri(matches) {
    Ok(ampq_uri) => {
      if let Err(error) = get_worker_status(ampq_uri, matches.value_of("worker_id"), 1000, false)
      {
        error!("{}", error);
      }
    }
    Err(error) => error!("Invalid RabbitMQ server URL: {}", error),
  }
}

pub fn watch(matches: &ArgMatches) {
  let interval_ms = match u64::from_str(matches.value_of("interval").unwrap()) {
    Ok(interval) => interval,
    Err(error) => {
      error!("Invalid interval value: {}", error.to_string());
      exit(-1);
    }
  };

  match get_amqp_server_uri(matches) {
    Ok(ampq_uri) => {
      if let Err(error) = get_worker_status(
        ampq_uri,
        matches.value_of("worker_id"),
        interval_ms,
        true,
      ) {
        error!("{}", error);
      }
    }
    Err(error) => error!("Invalid RabbitMQ server URL: {}", error),
  }
}

pub fn get_worker_status(
  amqp_uri: AMQPUri,
  worker_id: Option<&str>,
  interval_ms: u64,
  keep_watching: bool,
) -> Result<(), String> {
  let (sender, receiver) = mpsc::channel();

  let conn = Connection::connect_uri(
    amqp_uri,
    ConnectionProperties::default().with_default_executor(8),
  )
  .wait()
  .map_err(|e| e.to_string())?;

  let channel = conn.create_channel().wait().map_err(|e| e.to_string())?;

  declare_consumed_queues(&channel);

  let cloned_channel = channel.clone();

  let worker_statuses = Arc::new(Mutex::new(BTreeMap::new()));

  debug!("Start worker status consumer...");

  let cloned_worker_statuses = worker_statuses.clone();
  std::thread::spawn(move || {
    if let Err(error) = start_consumer(&cloned_channel, receiver, cloned_worker_statuses) {
      error!("{}", error);
    }
  });

  debug!("Start requesting worker status...");

  let headers = get_request_headers(worker_id);
  let mut max_displayed_workers = 0;

  println!(
    "{:2} {:<36} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
    " #",
    "Worker ID",
    "Used Memory",
    "Total Memory",
    "Used Swap",
    "Total Swap",
    "Nb. CPUs",
    "Activity",
    "Status"
  );

  loop {
    let result = send_status_request(&channel, headers.clone());
    debug!("Published status message: {:?}", result);

    std::thread::sleep(Duration::from_millis(interval_ms));

    print_worker_statuses(
      worker_statuses.clone(),
      max_displayed_workers,
      keep_watching,
    )?;
    max_displayed_workers = max_displayed_workers.max(worker_statuses.lock().unwrap().len());
    worker_statuses.lock().unwrap().clear();

    if !keep_watching {
      // Stop consuming thread;
      sender.send(()).map_err(|e| e.to_string())?;
      break;
    }
  }

  Ok(())
}

fn get_amqp_server_uri(matches: &ArgMatches) -> Result<AMQPUri, String> {
  let scheme = if matches.is_present("tls") {
    AMQPScheme::AMQPS
  } else {
    AMQPScheme::AMQP
  };

  let amqp_hostname = matches.value_of("host").unwrap();
  let amqp_port = matches.value_of("port").unwrap();
  let amqp_username = matches.value_of("user").unwrap();
  let amqp_password = matches.value_of("password").unwrap();
  let amqp_vhost = matches.value_of("virtual_host").unwrap();
  let amqp_vhost = format!("/{}", amqp_vhost);

  log::info!("Start connection with configuration:");
  log::info!("AMQP TLS: {:?}", scheme);
  log::info!("AMQP HOSTNAME: {}", amqp_hostname);
  log::info!("AMQP PORT: {}", amqp_port);
  log::info!("AMQP USERNAME: {}", amqp_username);
  log::info!("AMQP VIRTUAL HOST: {}", amqp_vhost);

  Ok(AMQPUri {
    scheme,
    authority: AMQPAuthority {
      userinfo: AMQPUserInfo {
        username: amqp_username.to_string(),
        password: amqp_password.to_string(),
      },
      host: amqp_hostname.to_string(),
      port: amqp_port.parse::<u16>().unwrap(),
    },
    vhost: amqp_vhost,
    query: Default::default(),
  })
}

fn declare_consumed_queues(channel: &Channel) {
  ExchangeDescription::new(EXCHANGE_NAME_WORKER_RESPONSE, ExchangeKind::Topic)
    .with_alternate_exchange(WORKER_RESPONSE_NOT_FOUND)
    .declare(channel);

  declare_queue(channel, QUEUE_WORKER_CREATED);
  declare_queue(channel, QUEUE_WORKER_INITIALIZED);
  declare_queue(channel, QUEUE_WORKER_STARTED);
  declare_queue(channel, QUEUE_WORKER_STATUS);
  declare_queue(channel, QUEUE_WORKER_TERMINATED);
  declare_queue(channel, QUEUE_WORKER_UPDATED);
}

fn declare_queue(channel: &Channel, queue: &str) {
  QueueDescription {
    name: queue.to_string(),
    durable: true,
    .. Default::default()
  }.declare(&channel);

  BindDescription {
    exchange: EXCHANGE_NAME_WORKER_RESPONSE.to_string(),
    queue: queue.to_string(),
    routing_key: queue.to_string(),
    headers: HashMap::new(),
  }.declare(&channel);

}

fn send_status_request(channel: &Channel, headers: FieldTable) -> Result<PublisherConfirm, String> {
  let status_message = serde_json::to_string(&OrderMessage::Status).map_err(|e| e.to_string())?;

  channel
    .basic_publish(
      EXCHANGE_NAME_DIRECT_MESSAGING,
      "mcai_workers_status",
      BasicPublishOptions::default(),
      status_message.as_bytes().to_vec(),
      BasicProperties::default().with_headers(headers),
    )
    .wait()
    .map_err(|e| e.to_string())
}

fn stop_consumer(rx: &Receiver<()>) -> bool {
  match rx.try_recv() {
    Ok(_) | Err(TryRecvError::Disconnected) => {
      return true;
    }
    Err(TryRecvError::Empty) => {}
  }
  false
}

fn start_consumer(
  channel: &Channel,
  rx: Receiver<()>,
  worker_statuses: Arc<Mutex<BTreeMap<String, ProcessStatus>>>,
) -> Result<(), String> {
  let mut status_consumer = channel
    .basic_consume(
      QUEUE_WORKER_STATUS,
      "mcai_workers_status_consumer",
      BasicConsumeOptions::default(),
      FieldTable::default(),
    )
    .wait()
    .map_err(|e| format!("Could not start consuming: {}", e.to_string()))?;

  while let Some(delivery) = futures_executor::block_on(status_consumer.next()) {
    if let Ok((channel, delivery)) = delivery {
      let message_data = std::str::from_utf8(&delivery.data).map_err(|e| e.to_string())?;
      debug!("Consumed message: {:?}", message_data);

      let process_status: ProcessStatus = serde_json::from_str(message_data)
        .map_err(|e| format!("Could not handle worker status: {}", e.to_string()))?;

      worker_statuses.lock().unwrap().insert(
        process_status
          .worker
          .system_info
          .docker_container_id
          .clone(),
        process_status.clone(),
      );

      channel
        .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
        .wait()
        .map_err(|e| format!("Could not ack message: {}", e.to_string()))?;
    }

    if stop_consumer(&rx) {
      debug!("Stop consuming.");
      break;
    }
  }

  Ok(())
}

fn print_worker_statuses(
  worker_statuses: Arc<Mutex<BTreeMap<String, ProcessStatus>>>,
  max_displayed_workers: usize,
  keep_watching: bool,
) -> Result<(), String> {
  let mut stdout = stdout();

  let worker_statuses = worker_statuses.lock().unwrap();
  let nb_workers = worker_statuses.len();
  let nb_displayed_lines = nb_workers.max(max_displayed_workers);
  let empty_lines = nb_displayed_lines - nb_workers;

  let mut cursor_position = 0;
  for (worker_index, (worker_id, process_status)) in worker_statuses.iter().enumerate() {
    let used_memory = process_status.worker.system_info.used_memory.to_string();
    let total_memory = process_status.worker.system_info.total_memory.to_string();
    let used_swap = process_status.worker.system_info.used_swap.to_string();
    let total_swap = process_status.worker.system_info.total_swap.to_string();
    let number_of_processors = process_status
      .worker
      .system_info
      .number_of_processors
      .to_string();
    let activity = format!("{:?}", process_status.worker.activity);

    let status = if let Some(job_result) = &process_status.job {
      // serde_json::to_string(job_result.get_status()).unwrap()
      job_result.get_status().to_string()
    } else {
      "-".to_string()
    };

    stdout
      .write(
        format!(
          "{:2} {:<36} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}\n",
          worker_index + 1,
          worker_id,
          used_memory,
          total_memory,
          used_swap,
          total_swap,
          number_of_processors,
          activity,
          status.as_str()
        )
        .as_bytes(),
      )
      .map_err(|e| e.to_string())?;
    cursor_position += 1;
  }

  for _l in 0..empty_lines {
    stdout
      .queue(Clear(ClearType::CurrentLine))
      .map_err(|e| e.to_string())?;
    stdout
      .queue(cursor::MoveToNextLine(1))
      .map_err(|e| e.to_string())?;
    cursor_position += 1;
  }

  stdout.flush().map_err(|e| e.to_string())?;

  if cursor_position > 0 && keep_watching {
    stdout
      .queue(cursor::MoveToPreviousLine(cursor_position as u16))
      .map_err(|e| e.to_string())?;
  }

  Ok(())
}
