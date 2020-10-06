use crate::PROJECT_NAME;
use directories::ProjectDirs;
use semver::Version;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct McaiWorkersConfig {
  pub rust_version: Option<Version>,
  pub mcai_sdk_version: Option<Version>,
  pub repos: Vec<RepoConfig>,
}

impl McaiWorkersConfig {
  pub fn open() -> Self {
    confy::load(PROJECT_NAME).unwrap()
  }

  pub fn store(&self) {
    confy::store(PROJECT_NAME, self).unwrap();

    let project = ProjectDirs::from("rs", "", "mcai-workers").unwrap();
    println!(
      "Stored configuration in folder: {}",
      project.preference_dir().display()
    );
  }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RepoConfig {
  pub name: String,
  pub provider: Provider,
  // list of Cargo.toml contents for this repository
  pub manifest_contents: Vec<String>,
  // list of Cargo.toml filenames for this repository
  pub manifest_filenames: Vec<String>,
  // list of Dockerfile contents for this repository
  pub docker_contents: Vec<String>,
  // list of Dockerfile filenames for this repository
  pub docker_filenames: Vec<String>,
}

impl RepoConfig {
  pub fn new(provider: Provider, name: &str) -> Self {
    RepoConfig {
      name: name.to_string(),
      provider,
      manifest_contents: vec![],
      manifest_filenames: vec![],
      docker_contents: vec![],
      docker_filenames: vec![],
    }
  }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Provider {
  Github,
  Gitlab,
}

impl Default for Provider {
  fn default() -> Self {
    Provider::Github
  }
}

impl From<&str> for Provider {
  fn from(value: &str) -> Self {
    match value {
      "github" => Provider::Github,
      "gitlab" => Provider::Gitlab,
      _ => panic!("Invalid provider"),
    }
  }
}
