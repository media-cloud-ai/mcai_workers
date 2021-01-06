use std::collections::{BTreeMap, HashMap};
use std::io::{stdout, Write};
use std::process::exit;
use std::str::FromStr;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use crate::amqp::{
  get_amqp_server_url, get_request_headers, DIRECT_MESSAGING, DIRECT_MESSAGING_RESPONSE,
};
use clap::ArgMatches;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, QueueableCommand};
use futures_util::stream::StreamExt;
use lapin::options::BasicAckOptions;
use lapin::publisher_confirm::PublisherConfirm;
use lapin::{
  options::{BasicConsumeOptions, BasicPublishOptions},
  types::FieldTable,
  uri::AMQPUri,
  BasicProperties, Connection, ConnectionProperties,
};
use mcai_worker_sdk::processor::ProcessStatus;
use mcai_worker_sdk::{
  debug, error,
  message_exchange::{
    rabbitmq::{
      channels::{BindDescription, QueueDescription},
      EXCHANGE_NAME_WORKER_RESPONSE, ROUTING_KEY_WORKER_STATUS,
    },
    OrderMessage,
  },
  Channel,
};

pub fn status(matches: &ArgMatches) {
  match get_amqp_server_url(matches) {
    Ok(server_url) => {
      if let Err(error) = get_worker_status(&server_url, matches.value_of("worker_id"), 1000, false)
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

  match get_amqp_server_url(matches) {
    Ok(server_url) => {
      if let Err(error) = get_worker_status(
        &server_url,
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
  server_url: &str,
  worker_id: Option<&str>,
  interval_ms: u64,
  keep_watching: bool,
) -> Result<(), String> {
  debug!("Try to connect to {}", server_url);

  let amqp_uri = AMQPUri::from_str(server_url)?;

  let (tx, rx) = mpsc::channel();

  let conn = Connection::connect_uri(
    amqp_uri,
    ConnectionProperties::default().with_default_executor(8),
  )
  .wait()
  .map_err(|e| e.to_string())?;

  let channel = conn.create_channel().wait().map_err(|e| e.to_string())?;

  declare_consumed_queue(&channel);

  let cloned_channel = channel.clone();

  let worker_statuses = Arc::new(Mutex::new(BTreeMap::new()));

  debug!("Start worker status consumer...");

  let cloned_worker_statuses = worker_statuses.clone();
  std::thread::spawn(move || {
    if let Err(error) = start_consumer(&cloned_channel, rx, cloned_worker_statuses) {
      error!("{}", error);
    }
  });

  debug!("Start requesting worker status...");

  let headers = get_request_headers(worker_id);
  let mut max_displayed_workers = 0;

  println!(
    " #  {:<36} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}",
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
      tx.send(()).map_err(|e| e.to_string())?;
      break;
    }
  }

  Ok(())
}

fn declare_consumed_queue(channel: &Channel) {
  let direct_messaging_response_queue = QueueDescription {
    name: DIRECT_MESSAGING_RESPONSE.to_string(),
    durable: true,
    auto_delete: false,
    dead_letter_exchange: None,
    dead_letter_routing_key: None,
    max_priority: None,
    message_ttl: None,
  };
  direct_messaging_response_queue.declare(&channel);

  let direct_messaging_response_bind = BindDescription {
    exchange: EXCHANGE_NAME_WORKER_RESPONSE.to_string(),
    queue: DIRECT_MESSAGING_RESPONSE.to_string(),
    routing_key: ROUTING_KEY_WORKER_STATUS.to_string(),
    headers: HashMap::new(),
  };
  direct_messaging_response_bind.declare(&channel);
}

fn send_status_request(channel: &Channel, headers: FieldTable) -> Result<PublisherConfirm, String> {
  let status_message = serde_json::to_string(&OrderMessage::Status).map_err(|e| e.to_string())?;

  channel
    .basic_publish(
      DIRECT_MESSAGING,
      "mcai_worker_status",
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
      DIRECT_MESSAGING_RESPONSE,
      "mcai_worker_status_consumer",
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
      serde_json::to_string(job_result.get_status()).unwrap()
    } else {
      "-".to_string()
    };

    stdout
      .write(
        format!(
          "{:2}. {:<36} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16} {:>16}\n",
          worker_index + 1,
          worker_id,
          used_memory,
          total_memory,
          used_swap,
          total_swap,
          number_of_processors,
          activity,
          status
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
