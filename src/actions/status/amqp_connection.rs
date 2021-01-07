use amq_protocol_uri::{AMQPAuthority, AMQPScheme, AMQPUri, AMQPUserInfo};
use clap::ArgMatches;
use futures_util::StreamExt;
use lapin::{
  options::{BasicAckOptions, BasicConsumeOptions, BasicPublishOptions},
  types::{AMQPValue, FieldTable},
  BasicProperties, Connection, ConnectionProperties, ExchangeKind
};
use mcai_worker_sdk::{
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
use std::{
  collections::HashMap,
  sync::mpsc::Sender,
};

pub struct AmqpConnection {
  channel: Channel,
  sender: Sender<ProcessStatus>,
}

impl AmqpConnection {
  pub fn new(arguments: &ArgMatches, sender: Sender<ProcessStatus>) -> Result<Self, String> {
    let amqp_uri = Self::parse_arguments(arguments)?;

    let connection = Connection::connect_uri(
      amqp_uri.clone(),
      ConnectionProperties::default().with_default_executor(8),
    )
    .wait()
    .map_err(|e| e.to_string())?;

    let channel = connection.create_channel().wait().map_err(|e| e.to_string())?;

    Self::declare_consumed_queues(&channel);

    Ok(AmqpConnection {
      channel,
      sender
    })
  }

  pub fn start_consumer(&self) {
    let channel = self.channel.clone();
    let sender = self.sender.clone();

    std::thread::spawn(move || {
      let mut status_consumer = channel
        .basic_consume(
          QUEUE_WORKER_STATUS,
          "mcai_workers_status_consumer",
          BasicConsumeOptions::default(),
          FieldTable::default(),
        )
        .wait().unwrap();

      while let Some(delivery) = futures_executor::block_on(status_consumer.next()) {
        if let Ok((channel, delivery)) = delivery {
          let message_data = std::str::from_utf8(&delivery.data).unwrap();
          log::debug!("Consuming message: {:?}", message_data);

          let process_status: ProcessStatus = serde_json::from_str(message_data).unwrap();

          sender.send(process_status).unwrap();

          channel
            .basic_ack(delivery.delivery_tag, BasicAckOptions::default())
            .wait().unwrap();
        }
      }
    });
  }

  pub fn send_status_request(&self, worker_ids: Vec<&str>) -> Result<(), String> {
    let status_message = serde_json::to_string(&OrderMessage::Status).map_err(|e| e.to_string())?;

    if worker_ids.is_empty() {
      let mut headers = FieldTable::default();
      headers.insert("broadcast".into(), AMQPValue::Boolean(true));

      self.channel
        .basic_publish(
          EXCHANGE_NAME_DIRECT_MESSAGING,
          "mcai_workers_status",
          BasicPublishOptions::default(),
          status_message.as_bytes().to_vec(),
          BasicProperties::default().with_headers(headers),
        )
        .wait()
        .map_err(|e| e.to_string())?;

      return Ok(());
    }

    for worker_id in worker_ids {
      let mut headers = FieldTable::default();
      headers.insert(
        "worker_name".into(),
        AMQPValue::LongString(worker_id.to_string().into()),
      );

      self.channel
        .basic_publish(
          EXCHANGE_NAME_DIRECT_MESSAGING,
          "mcai_workers_status",
          BasicPublishOptions::default(),
          status_message.as_bytes().to_vec(),
          BasicProperties::default().with_headers(headers),
        )
        .wait()
        .map_err(|e| e.to_string())?;

    }

    Ok(())
  }

  fn parse_arguments(arguments: &ArgMatches) -> Result<AMQPUri, String> {
    let scheme = if arguments.is_present("tls") {
      AMQPScheme::AMQPS
    } else {
      AMQPScheme::AMQP
    };

    let amqp_hostname = arguments.value_of("hostname").unwrap();
    let amqp_port = arguments.value_of("port").unwrap();
    let amqp_username = arguments.value_of("username").unwrap();
    let amqp_password = arguments.value_of("password").unwrap();
    let amqp_vhost = arguments.value_of("virtual_host").unwrap();

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
      vhost: amqp_vhost.to_string(),
      query: Default::default(),
    })
  }

  fn declare_consumed_queues(channel: &Channel) {
    ExchangeDescription::new(EXCHANGE_NAME_WORKER_RESPONSE, ExchangeKind::Topic)
      .with_alternate_exchange(WORKER_RESPONSE_NOT_FOUND)
      .declare(channel);

    Self::declare_queue(channel, QUEUE_WORKER_CREATED);
    Self::declare_queue(channel, QUEUE_WORKER_INITIALIZED);
    Self::declare_queue(channel, QUEUE_WORKER_STARTED);
    Self::declare_queue(channel, QUEUE_WORKER_STATUS);
    Self::declare_queue(channel, QUEUE_WORKER_TERMINATED);
    Self::declare_queue(channel, QUEUE_WORKER_UPDATED);
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
}