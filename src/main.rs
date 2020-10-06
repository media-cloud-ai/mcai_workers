#[macro_use]
extern crate serde_derive;

mod config;
mod github;
mod gitlab;

use crate::gitlab::Gitlab;
use cargo_toml::Manifest;
use clap::{App, Arg, SubCommand};
use colored::*;
use console::Emoji;
use directories::ProjectDirs;
use dockerfile_parser::Dockerfile;
use github::Github;

static PROJECT_NAME: &str = "mcai-workers";

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
    .get_matches();

  let mut cfg: config::McaiWorkersConfig = confy::load(PROJECT_NAME).unwrap();

  if let Some(matches) = matches.subcommand_matches("register") {
    let repository = matches.value_of("repository").unwrap();
    let provider: config::Provider = matches.value_of("provider").unwrap().into();

    let repo_config = config::RepoConfig::new(provider, repository);
    cfg.repos.push(repo_config);
    confy::store(PROJECT_NAME, cfg).unwrap();

    let project = ProjectDirs::from("rs", "", "mcai-workers").unwrap();
    println!(
      "Stored configuration in folder: {}",
      project.preference_dir().display()
    );
    return;
  }

  if let Some(matches) = matches.subcommand_matches("unregister") {
    let repository = matches.value_of("repository").unwrap();
    let provider: config::Provider = matches.value_of("provider").unwrap().into();

    let repos = cfg
      .repos
      .iter()
      .filter(|repo| repo.name != repository && repo.provider == provider)
      .cloned()
      .collect();

    cfg.repos = repos;

    confy::store(PROJECT_NAME, cfg).unwrap();

    let project = ProjectDirs::from("rs", "", "mcai-workers").unwrap();
    println!(
      "Stored configuration in folder: {}",
      project.preference_dir().display()
    );
    return;
  }

  if matches.subcommand_matches("show").is_some() {
    for repo in &cfg.repos {
      let provider = format!("{:?}", repo.provider);
      println!("{} {}", provider.green().bold(), repo.name);
    }
    return;
  }

  if let Some(matches) = matches.subcommand_matches("fetch") {
    for repo in cfg.repos.iter_mut() {
      println!("Fetch {}", repo.name);
      match repo.provider {
        config::Provider::Github => {
          let token = matches.value_of("github-token").unwrap();
          let github = Github::new(token);

          let cargo_toml_manifest = github.get_file_content(&repo.name, "Cargo.toml");
          repo.cargo_toml_manifest = cargo_toml_manifest;

          repo.docker_contents.clear();
          if let Some(dockerfile) = github.get_file_content(&repo.name, "Dockerfile") {
            repo.docker_contents.push(dockerfile);
          }
        }
        config::Provider::Gitlab => {
          let token = matches.value_of("gitlab-token").unwrap();
          let gitlab = Gitlab::new(token);

          let cargo_toml_manifest = gitlab.get_file_content(&repo.name, "Cargo.toml");
          repo.cargo_toml_manifest = cargo_toml_manifest;

          repo.docker_contents.clear();
          if let Some(dockerfile) = gitlab.get_file_content(&repo.name, "Dockerfile") {
            repo.docker_contents.push(dockerfile);
          }
        }
      }
    }
    confy::store(PROJECT_NAME, cfg).unwrap();
    return;
  }

  if let Some(matches) = matches.subcommand_matches("list") {
    for repo in &cfg.repos {
      println!();
      println!(
        "{} {}",
        Emoji("🚀", &"=>".green().bold()),
        repo.name.green().bold()
      );

      if let Some(cargo_content) = &repo.cargo_toml_manifest {
        let manifest = Manifest::from_str(&cargo_content).unwrap();
        if matches.is_present("dependencies") {
          for (name, version) in &manifest.dependencies {
            let version = match version {
              cargo_toml::Dependency::Simple(version) => version.to_string(),
              version => format!("{:?}", version),
            };

            println!("  - {} ({})", name, version);
          }
        }
      }

      for dockerfile in &repo.docker_contents {
        let dockerfile = Dockerfile::parse(&dockerfile).unwrap();
        println!(
          "  {} {}",
          Emoji("🐳", &"=>".cyan().bold()),
          docker_information(&dockerfile).cyan()
        );
      }
    }

    return;
  }
}

fn docker_information(dockerfile: &Dockerfile) -> String {
  use dockerfile_parser::{ImageRef, Instruction};

  let images = dockerfile
    .instructions
    .iter()
    .filter(|instruction| matches!(instruction, Instruction::From(_)))
    .map(|instruction| {
      if let Instruction::From(content) = instruction {
        content.image_parsed.clone()
      } else {
        unreachable!()
      }
    })
    .collect::<Vec<ImageRef>>();

  for image in &images {
    let reference_images = vec![
      "rust",
      "mediacloudai/rs_command_line_worker",
      "mediacloudai/py_mcai_worker_sdk",
      "mediacloudai/c_mcai_worker_sdk",
    ];

    if !reference_images.contains(&image.image.as_str()) {
      continue;
    }

    let reference_image = match image.image.as_str() {
      "rust" => "Rust",
      "mediacloudai/rs_command_line_worker" => "Command Line",
      "mediacloudai/py_mcai_worker_sdk" => "Python MCAI SDK",
      "mediacloudai/c_mcai_worker_sdk" => "C MCAI SDK",
      _ => unreachable!(),
    };

    return format!(
      "{} {}",
      reference_image,
      image.tag.as_ref().unwrap_or(&"latest".to_string())
    );
  }
  "".to_string()
}
