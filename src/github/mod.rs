mod content;
mod license;
mod owner;
mod permissions;
mod repo;

use content::Content;
use license::License;
use owner::Owner;
use permissions::Permissions;
use std::str;

use github_rs::client::{Executor, Github as GithubClient};

pub struct Github {
  client: GithubClient,
}

impl Github {
  pub fn new(token: &str) -> Github {
    let client = GithubClient::new(token).unwrap();

    Github { client }
  }

  pub fn get_file_content(&self, repository: &str, filename: &str) -> Option<String> {
    let (organization, repo_name) = self.get_repo_information(repository);

    let (_, status, response) = self
      .client
      .get()
      .repos()
      .owner(&organization)
      .repo(&repo_name)
      .contents()
      .path(filename)
      .execute::<serde_json::Value>()
      .unwrap();

    if status == 200 {
      let response: Content = serde_json::from_value(response.unwrap()).unwrap();
      let content = base64::decode(response.content.replace("\n", "")).unwrap();
      Some(str::from_utf8(&content).unwrap().to_string())
    } else {
      None
    }
  }

  fn get_repo_information(&self, repository: &str) -> (String, String) {
    let repository = repository.split('/').collect::<Vec<&str>>();

    let organization = repository[0].to_string();
    let name = repository[1].to_string();
    (organization, name)
  }
}
