#[macro_use]
extern crate serde_derive;

mod actions;
mod config;
mod dockerhub;
mod github;
mod gitlab;

use cargo_toml::Manifest;
use clap::{App, Arg, SubCommand};
use dockerfile_parser::Dockerfile;

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
    .subcommand(
      SubCommand::with_name("benchmark")
        .about("Benchmark a video using the docker image of it.")
        .version("0.1")
        .arg(
          Arg::with_name("worker")
            .index(1)
            .takes_value(true)
            .required(true),
        )
        .arg(
          Arg::with_name("example")
            .index(2)
            .takes_value(true)
            .required(true),
        )
        .arg(
          Arg::with_name("cpus")
            .default_value("1")
            .short("c")
            .long("cpus")
            .takes_value(true),
        )
        .arg(
          Arg::with_name("envs")
            .help("Json of envs representing files to mount on container (e.g. {'SOURCE_PATH': '/path'})")
            .default_value("{}")
            .short("e")
            .long("envs")
            .takes_value(true)
            .validator(valid_json),
        )
        .arg(
          Arg::with_name("memory")
            .default_value("5000000000")
            .short("m")
            .long("memory")
            .takes_value(true),
        )
        .arg(
          Arg::with_name("nepoch")
            .help("Number of epoch to benchmark the worker.")
            .default_value("1")
            .short("n")
            .long("nepoch")
            .takes_value(true),
        )
        .arg(
          Arg::with_name("tag")
            .default_value("latest")
            .short("t")
            .long("tag")
            .takes_value(true)
            .multiple(true),
        )
        .arg(github_token_arg.clone())
        .arg(gitlab_token_arg.clone()),
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
          Arg::with_name("image")
            .short("i")
            .long("image")
            .takes_value(true)
            .required(true),
        )
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
          Arg::with_name("registry")
            .long("registry")
            .takes_value(true)
            .possible_values(&["dockerhub", "gitlab"])
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
    .get_matches();

  let mut cfg = config::McaiWorkersConfig::open();

  if let Some(matches) = matches.subcommand_matches("benchmark") {
    actions::benchmark(&mut cfg, matches);
    return;
  }

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
}

fn valid_json(envs: String) -> Result<(), String> {
  match serde_json::from_str::<std::collections::HashMap<String, String>>(envs.as_str()) {
    Ok(_) => Ok(()),
    Err(error) => Err(format!("{}", error)),
  }
}
