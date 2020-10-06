use crate::config::{McaiWorkersConfig, Provider};
use clap::ArgMatches;

pub fn unregister<'a>(cfg: &mut McaiWorkersConfig, matches: &ArgMatches<'a>) {
  let repository = matches.value_of("repository").unwrap();
  let provider: Provider = matches.value_of("provider").unwrap().into();

  let repos = cfg
    .repos
    .iter()
    .filter(|repo| repo.name != repository && repo.provider == provider)
    .cloned()
    .collect();

  cfg.repos = repos;

  cfg.store();
}
