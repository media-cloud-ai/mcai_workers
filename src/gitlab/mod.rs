use gitlab::Gitlab as GitlabClient;
use std::str;

pub struct Gitlab {
  token: String,
  client: GitlabClient,
}

impl Gitlab {
  pub fn new(token: &str) -> Gitlab {
    let client = GitlabClient::new("gitlab.com", token).unwrap();

    Gitlab {
      client,
      token: token.to_string(),
    }
  }

  pub fn get_file_content(&self, repository: &str, filename: &str) -> Option<String> {
    use {
      gitlab::api::{projects, Query},
      reqwest::{
        blocking::Client,
        header::{HeaderMap, HeaderValue},
      },
    };

    let endpoint = projects::Project::builder()
      .project(repository)
      .build()
      .unwrap();

    let project: Project = endpoint.query(&self.client).unwrap();

    let url = format!(
      "https://gitlab.com/api/v4/projects/{}/repository/files/{}?ref=master",
      project.id, filename
    );

    let mut headers = HeaderMap::new();
    headers.insert("PRIVATE-TOKEN", HeaderValue::from_str(&self.token).unwrap());

    let client = Client::builder().default_headers(headers).build().unwrap();

    if let Ok(response) = client.get(&url).send().unwrap().json::<FileResponse>() {
      let content = base64::decode(response.content).unwrap();
      return Some(str::from_utf8(&content).unwrap().to_string());
    }

    None
  }
}

#[derive(Debug, Deserialize)]
struct Project {
  name: String,
  id: usize,
}

#[derive(Debug, Deserialize)]
struct FileResponse {
  file_name: String,
  file_path: String,
  size: usize,
  encoding: String,
  content: String,
}
