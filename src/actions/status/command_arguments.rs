use clap::Arg;

pub fn get_command_arguments() -> Vec<Arg<'static, 'static>> {
  vec![
    Arg::with_name("hostname")
      .long("hostname")
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
      .default_value("/"),
    Arg::with_name("username")
      .short("u")
      .long("username")
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
      .takes_value(true)
      .multiple(true),
  ]
}
