mod amqp_connection;
mod command_arguments;
mod worker_statuses;

pub use command_arguments::get_command_arguments;

use amqp_connection::AmqpConnection;
use clap::ArgMatches;
use std::process::exit;
use std::str::FromStr;
use std::time::Duration;
use worker_statuses::WorkerStatuses;

pub fn status(arguments: &ArgMatches) {
  let mut worker_statuses = WorkerStatuses::new();
  let sender = worker_statuses.get_sender();

  let connection = AmqpConnection::new(arguments, sender).unwrap();
  connection.start_consumer();
  connection.send_status_request(vec![]).unwrap();

  std::thread::sleep(Duration::from_millis(3000));
  worker_statuses.dump().unwrap();
}

pub fn watch(arguments: &ArgMatches) {
  let interval_ms = match u64::from_str(arguments.value_of("interval").unwrap()) {
    Ok(interval) => interval,
    Err(error) => {
      log::error!("Invalid interval value: {}", error.to_string());
      exit(-1);
    }
  };

  let mut worker_statuses = WorkerStatuses::new();
  worker_statuses.set_keep_watching();
  let sender = worker_statuses.get_sender();

  let connection = AmqpConnection::new(arguments, sender).unwrap();
  connection.start_consumer();

  loop {
    connection.send_status_request(vec![]).unwrap();
    std::thread::sleep(Duration::from_millis(interval_ms));
    worker_statuses.dump().unwrap();
  }
}
