use crate::{
  config::{McaiWorkersConfig, Provider, RepoConfig},
  PROJECT_NAME,
};
use clap::ArgMatches;
use directories::ProjectDirs;

pub fn register<'a>(cfg: &mut McaiWorkersConfig, matches: &ArgMatches<'a>) {
  let repository = matches.value_of("repository").unwrap();
  let provider: Provider = matches.value_of("provider").unwrap().into();

  let mut repo_config = RepoConfig::new(provider, repository);

  if let Some(values) = matches.values_of("manifest") {
    for value in values {
      repo_config.manifest_filenames.push(value.to_string());
    }
  }

  if let Some(values) = matches.values_of("dockerfile") {
    for value in values {
      repo_config.docker_filenames.push(value.to_string());
    }
  }

  cfg.repos.push(repo_config);
  confy::store(PROJECT_NAME, cfg).unwrap();

  let project = ProjectDirs::from("rs", "", "mcai-workers").unwrap();
  println!(
    "Stored configuration in folder: {}",
    project.preference_dir().display()
  );
}
