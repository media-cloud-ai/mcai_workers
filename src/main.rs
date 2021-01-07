#[macro_use]
extern crate serde_derive;

mod actions;
mod config;
mod github;
mod gitlab;

use cargo_toml::Manifest;
use clap::{App, Arg, SubCommand};
use dockerfile_parser::Dockerfile;

use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;

static PROJECT_NAME: &str = "mcai-workers";
static OPEN_SOURCE_WORKERS_URL: &str = "https://raw.githubusercontent.com/media-cloud-ai/mcai_workers/master/workers/open_source_mcai_workers.json";

#[derive(Debug)]
pub struct Repository {
  name: String,
  cargo_manifest: Option<Manifest>,
  dockerfile: Option<Dockerfile>,
}

fn main() {
  let github_token_arg = Arg::with_name("github-token")
    .long("github-token")
    .env("GITHUB_TOKEN");

  let gitlab_token_arg = Arg::with_name("gitlab-token")
    .long("gitlab-token")
    .env("GITLAB_TOKEN");

  let matches = App::new(PROJECT_NAME)
    .arg(
      Arg::with_name("verbose")
        .help("Enable debug logging.")
        .short("v")
        .long("verbose"),
    )
    .subcommand(
      SubCommand::with_name("register-all")
        .about("Register a repositories form a content description")
        .version("0.1")
        .arg(
          Arg::with_name("url")
            .short("u")
            .long("url")
            .default_value(OPEN_SOURCE_WORKERS_URL)
            .takes_value(true)
            .multiple(true),
        ),
    )
    .subcommand(
      SubCommand::with_name("register")
        .about("Register a new repository to list of current workers")
        .version("0.1")
        .arg(
          Arg::with_name("repository")
            .short("r")
            .long("repository")
            .takes_value(true)
            .required(true),
        )
        .arg(
          Arg::with_name("provider")
            .short("p")
            .long("provider")
            .takes_value(true)
            .possible_values(&["github", "gitlab"])
            .required(true),
        )
        .arg(
          Arg::with_name("manifest")
            .short("m")
            .long("manifest")
            .takes_value(true)
            .multiple(true),
        )
        .arg(
          Arg::with_name("dockerfile")
            .short("d")
            .long("dockerfile")
            .takes_value(true)
            .multiple(true),
        ),
    )
    .subcommand(
      SubCommand::with_name("unregister")
        .about("Unregister a repository")
        .version("0.1")
        .arg(
          Arg::with_name("repository")
            .short("r")
            .long("repository")
            .takes_value(true)
            .required(true),
        )
        .arg(
          Arg::with_name("provider")
            .short("p")
            .long("provider")
            .takes_value(true)
            .possible_values(&["github", "gitlab"])
            .required(true),
        ),
    )
    .subcommand(
      SubCommand::with_name("show")
        .about("Display stored configuration")
        .version("0.1"),
    )
    .subcommand(
      SubCommand::with_name("fetch")
        .about("Update local cache from repositories")
        .version("0.1")
        .arg(github_token_arg.clone())
        .arg(gitlab_token_arg.clone()),
    )
    .subcommand(
      SubCommand::with_name("list")
        .about("List status of registered workers")
        .version("0.1")
        .arg(github_token_arg)
        .arg(gitlab_token_arg)
        .arg(
          Arg::with_name("dependencies")
            .short("d")
            .long("dependencies"),
        )
        .arg(
          Arg::with_name("exclude-sdk-versions")
            .short("e")
            .long("exclude-sdk-versions")
            .takes_value(true),
        ),
    )
    .subcommand(
      SubCommand::with_name("versions")
        .about("Set versions of tools (Rust, Media-Cloud-AI SDK)")
        .version("0.1")
        .arg(Arg::with_name("rust").long("rust").takes_value(true))
        .arg(
          Arg::with_name("mcai-sdk")
            .long("mcai-sdk")
            .takes_value(true),
        ),
    )
    .subcommand(
      SubCommand::with_name("status")
        .about("List workers status.")
        .version("0.1")
        .args(&actions::status::get_command_arguments())
    )
    .subcommand(
      SubCommand::with_name("watch")
        .about("Watch workers status.")
        .version("0.1")
        .args(&actions::status::get_command_arguments())
        .arg(
          Arg::with_name("interval")
            .short("n")
            .long("interval")
            .takes_value(true)
            .help("Watch interval in milliseconds")
            .default_value("5000"),
        )
    )
    .get_matches();

  let mut cfg = config::McaiWorkersConfig::open();

  // Logger

  let log_level = if matches.is_present("verbose") {
    LevelFilter::Debug
  } else {
    LevelFilter::Error
  };

  let mut builder = Builder::from_default_env();
  builder
    .format(move |stream, record| writeln!(stream, "[{}] {}", record.level(), record.args(),))
    .filter_level(log_level)
    .init();

  // Sub-commands

  if let Some(matches) = matches.subcommand_matches("register") {
    actions::register(&mut cfg, matches);
    return;
  }

  if let Some(matches) = matches.subcommand_matches("register-all") {
    actions::register_all(&mut cfg, matches);
    return;
  }

  if let Some(matches) = matches.subcommand_matches("unregister") {
    actions::unregister(&mut cfg, matches);
    return;
  }

  if matches.subcommand_matches("show").is_some() {
    actions::show(&cfg);
    return;
  }

  if let Some(matches) = matches.subcommand_matches("fetch") {
    actions::fetch(&mut cfg, matches);
    return;
  }

  if let Some(matches) = matches.subcommand_matches("list") {
    actions::list(&cfg, matches);
    return;
  }

  if let Some(matches) = matches.subcommand_matches("versions") {
    actions::versions(&mut cfg, matches);
    return;
  }

  if let Some(matches) = matches.subcommand_matches("status") {
    actions::status(matches);
    return;
  }

  if let Some(matches) = matches.subcommand_matches("watch") {
    actions::watch(matches);
    return;
  }
}
