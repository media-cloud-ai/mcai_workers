use crate::config::{McaiWorkersConfig, Provider, RepoConfig};
use clap::ArgMatches;
use reqwest::blocking::Client;
use semver::Version;
use std::fs;

pub fn register_all<'a>(cfg: &mut McaiWorkersConfig, matches: &ArgMatches<'a>) {
  let urls = matches.values_of("url").unwrap();

  let repositories : Vec<RepoConfig> =
    urls
      .map(|url| {
        fetch_content(&url)
      })
      .filter(|content| content.is_some())
      .map(|content| {
        let description : Description = serde_json::from_str(&content.unwrap()).unwrap();

        if let Some(mcai_sdk_version) = description.mcai_sdk_version {
          cfg.mcai_sdk_version = Some(mcai_sdk_version);
        }

        if let Some(rust_version) = description.rust_version {
          cfg.rust_version = Some(rust_version);
        }

        description.workers
          .iter()
          .map(|repo| {
            let mut rc = RepoConfig::new(repo.provider.clone(), &repo.name);

            rc.manifest_filenames = repo.manifests.clone();
            rc.docker_filenames = repo.dockerfiles.clone();

            rc
          })
          .collect::<Vec<RepoConfig>>()
      })
      .flatten()
      .collect();

  for repository in repositories {
    cfg.add_repo(repository);
  }

  cfg.store();
}

pub fn fetch_content(url : &str) -> Option<String> {
  if url.starts_with("http://") || url.starts_with("https://") {
    let client = Client::builder().build().unwrap();
    return client.get(url).send().unwrap().text().ok();
  }

  fs::read_to_string(url).ok()
}

#[derive(Debug, Deserialize)]
struct Description {
  mcai_sdk_version: Option<Version>,
  rust_version: Option<Version>,
  workers: Vec<Repository>,
}

#[derive(Debug, Deserialize)]
struct Repository {
  provider: Provider,
  name: String,
  #[serde(default)]
  manifests: Vec<String>,
  #[serde(default)]
  dockerfiles: Vec<String>,
}
