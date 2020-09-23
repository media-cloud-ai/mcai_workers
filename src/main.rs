#[macro_use]
extern crate serde_derive;

use clap::{App, Arg};
use colored::*;
use github_rs::client::{Executor, Github};

mod github;

fn main() {
  let matches = App::new("mcai-workers")
    .arg(
      Arg::with_name("github-token")
        .long("github-token")
        .env("GITHUB_TOKEN")
        .required(true),
    )
    .arg(Arg::with_name("dependencies").long("dependencies"))
    .arg(
      Arg::with_name("exclude-sdk-versions")
        .short("e")
        .long("exclude-sdk-versions")
        .takes_value(true),
    )
    .get_matches();

  let github_token = matches.value_of("github-token").unwrap();
  let dependencies = matches.is_present("dependencies");
  let exclude_sdk_versions: Vec<&str> = matches
    .values_of("exclude-sdk-versions")
    .unwrap_or_default()
    .collect();

  println!("Start listing workers");
  let client = Github::new(github_token).unwrap();

  if let Ok((_, _, repos)) = client
    .get()
    .orgs()
    .org("media-cloud-ai")
    .repos()
    .execute::<Vec<github::Repo>>()
  {
    if let Some(repos) = repos {
      for repo in repos.iter() {
        repo_status(&client, &repo, dependencies, &exclude_sdk_versions);
      }
    }
  }
}

fn repo_status(
  client: &Github,
  repo: &github::Repo,
  dependencies: bool,
  exclude_sdk_versions: &[&str],
) {
  if let Some(manifest) = get_cargo_toml(client, repo) {
    let filtered_dependencies: Vec<(&String, &cargo_toml::Dependency)> = manifest
      .dependencies
      .iter()
      .filter(|(name, version)| {
        name.as_str() == "mcai_worker_sdk" && !exclude_sdk_versions.contains(&version.req())
      })
      .collect();

    if filtered_dependencies.is_empty() {
      return;
    }

    if let Some(package) = manifest.package {
      println!();
      let version = format!("(version {})", package.version);
      println!("{} {}", repo.name.green().bold(), version.italic());

      if dependencies {
        for (name, version) in &manifest.dependencies {
          let version = match version {
            cargo_toml::Dependency::Simple(version) => version.to_string(),
            version => format!("{:?}", version),
          };

          println!("  - {} ({})", name, version);
        }
      }
    }
  }
}

fn get_cargo_toml(client: &Github, repo: &github::Repo) -> Option<cargo_toml::Manifest> {
  let (_, status, response) = client
    .get()
    .repos()
    .owner("media-cloud-ai")
    .repo(&repo.name)
    .contents()
    .path("Cargo.toml")
    .execute::<serde_json::Value>()
    .unwrap();

  if status == 200 {
    let response: github::Content = serde_json::from_value(response.unwrap()).unwrap();
    let cargo_content = base64::decode(response.content.replace("\n", "")).unwrap();
    Some(cargo_toml::Manifest::from_slice(&cargo_content).unwrap())
  } else {
    None
  }
}
