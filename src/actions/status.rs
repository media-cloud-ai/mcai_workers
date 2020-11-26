use std::collections::HashMap;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use clap::ArgMatches;
use futures_util::stream::StreamExt;
use lapin::options::BasicAckOptions;
use lapin::{
  options::{BasicConsumeOptions, BasicPublishOptions},
  types::{AMQPValue, FieldTable},
  uri::AMQPUri,
  BasicProperties, Connection, ConnectionProperties,
};
use mcai_worker_sdk::message::control::DirectMessage;
use mcai_worker_sdk::{
  channels::{BindDescription, QueueDescription},
  debug, error,
  worker::system_information::SystemInformation,
};

const DIRECT_MESSAGING: &str = "direct_messaging";
const DIRECT_MESSAGING_RESPONSE: &str = "direct_messaging_response";

pub fn status(matches: &ArgMatches) {
  if let Some(server_url) = matches.value_of("url") {
    if let Err(error) = watch(server_url, matches.value_of("worker_id"), 1000, true) {
      error!("{}", error);
    }
  } else {
    error!("Unspecified RabbitMQ server url.");
  }
}

pub fn watch(
  server_url: &str,
  worker_id: Option<&str>,
  interval_ms: u64,
  once: bool,
) -> Result<(), String> {
  let amqp_uri = AMQPUri::from_str(server_url)?;

  let (tx, rx) = mpsc::channel();

  let conn = Connection::connect_uri(
    amqp_uri,
    ConnectionProperties::default().with_default_executor(8),
  )
  .wait()
  .map_err(|e| e.to_string())?;

  let channel = conn.create_channel().wait().map_err(|e| e.to_string())?;

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
    exchange: DIRECT_MESSAGING_RESPONSE.to_string(),
    queue: DIRECT_MESSAGING_RESPONSE.to_string(),
    routing_key: "*".to_string(),
    headers: HashMap::new(),
  };
  direct_messaging_response_bind.declare(&channel);

  let cloned_channel = channel.clone();

  std::thread::spawn(move || {
    let mut status_consumer = match cloned_channel
      .basic_consume(
        DIRECT_MESSAGING_RESPONSE,
        "mcai_worker_status_consumer",
        BasicConsumeOptions::default(),
        FieldTable::default(),
      )
      .wait()
    {
      Ok(consumer) => consumer,
      Err(error) => {
        error!("Could not start consuming: {}", error.to_string());
        return;
      }
    };

    while let Some(delivery) = futures_executor::block_on(status_consumer.next()) {
      if let Ok((channel, delivery)) = delivery {
        let message_data = std::str::from_utf8(&delivery.data).unwrap();
        debug!("Consumed message: {:?}", message_data);

        match serde_json::from_str::<SystemInformation>(message_data) {
          Ok(sys_info) => println!("{:?}", sys_info),
          Err(error) => error!("Could not handle worker status: {}", error.to_string()),
        }

        if let Err(error) = channel
          .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
          .wait()
        {
          error!("Could not ack message: {}", error.to_string())
        }
      }

      match rx.try_recv() {
        Ok(_) | Err(TryRecvError::Disconnected) => {
          debug!("Stop consuming.");
          break;
        }
        Err(TryRecvError::Empty) => {}
      }
    }
  });

  let mut headers = FieldTable::default();
  if let Some(worker) = worker_id {
    headers.insert(
      "worker_name".into(),
      AMQPValue::LongString(worker.to_string().into()),
    );
  } else {
    headers.insert("broadcast".into(), AMQPValue::Boolean(true));
  }

  debug!("Start requesting worker status...");

  loop {
    let status_message =
      serde_json::to_string(&DirectMessage::Status).map_err(|e| e.to_string())?;

    let result = channel
      .basic_publish(
        DIRECT_MESSAGING,
        "mcai_worker_status",
        BasicPublishOptions::default(),
        status_message.as_bytes().to_vec(),
        BasicProperties::default().with_headers(headers.clone()),
      )
      .wait()
      .map_err(|e| e.to_string())?;

    debug!("Published message {:?}: {:?}", status_message, result);

    std::thread::sleep(Duration::from_millis(interval_ms));

    if once {
      // Stop consuming thread;
      tx.send(()).map_err(|e| e.to_string())?;
      break;
    }
  }

  Ok(())
}

#[test]
pub fn test_status() {
  watch(
    "amqp://mediacloudai:mediacloudai@localhost:5678/media_cloud_ai_dev",
    None,
    true,
  );
}
