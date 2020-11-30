use clap::{Arg, ArgMatches};
use lapin::types::{AMQPValue, FieldTable};

pub(crate) const DIRECT_MESSAGING: &str = "direct_messaging";
pub(crate) const DIRECT_MESSAGING_RESPONSE: &str = "direct_messaging_response";

pub fn get_connection_command_args() -> [Arg<'static, 'static>; 6] {
  [
    Arg::with_name("host")
      .short("h")
      .long("host")
      .takes_value(true)
      .default_value("localhost"),
    Arg::with_name("port")
      .short("p")
      .long("port")
      .takes_value(true)
      .default_value("5672"),
    Arg::with_name("virtual_host")
      .short("vh")
      .long("vhost")
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
  ]
}

pub fn get_worker_id_argument(required: bool) -> Arg<'static, 'static> {
  Arg::with_name("worker_id")
    .short("w")
    .long("worker-id")
    .takes_value(true)
    .required(required)
}

pub fn get_amqp_server_url(matches: &ArgMatches) -> Result<String, String> {
  let scheme = if matches.is_present("tls") {
    "amqps"
  } else {
    "amqp"
  };

  let host = matches.value_of("host").unwrap();
  let port = matches.value_of("port").unwrap();
  let user = matches.value_of("user").unwrap();
  let password = matches.value_of("password").unwrap();
  let virtual_host = matches.value_of("virtual_host").unwrap();

  let server_url = format!(
    "{}://{}:{}@{}:{}/{}",
    scheme, user, password, host, port, virtual_host
  );

  Ok(server_url)
}

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

