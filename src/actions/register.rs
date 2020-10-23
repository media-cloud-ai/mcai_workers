use crate::config::{McaiWorkersConfig, Provider, RepoConfig};
use clap::ArgMatches;

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

  cfg.add_repo(repo_config);
  cfg.store();
}
