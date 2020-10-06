use crate::{
  config::{McaiWorkersConfig, Provider},
  PROJECT_NAME,
};
use clap::ArgMatches;
use directories::ProjectDirs;

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

  confy::store(PROJECT_NAME, cfg).unwrap();

  let project = ProjectDirs::from("rs", "", "mcai-workers").unwrap();
  println!(
    "Stored configuration in folder: {}",
    project.preference_dir().display()
  );
}
