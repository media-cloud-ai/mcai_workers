use crate::config::McaiWorkersConfig;
use clap::ArgMatches;
use semver::Version;

pub fn versions<'a>(cfg: &mut McaiWorkersConfig, matches: &ArgMatches<'a>) {
  if let Some(rust_version) = matches.value_of("rust") {
    if let Ok(rust_version) = Version::parse(rust_version) {
      cfg.rust_version = Some(rust_version);
    }
  }

  if let Some(mcai_sdk_version) = matches.value_of("mcai-sdk") {
    if let Ok(mcai_sdk_version) = Version::parse(mcai_sdk_version) {
      cfg.mcai_sdk_version = Some(mcai_sdk_version);
    }
  }

  cfg.store();
}
