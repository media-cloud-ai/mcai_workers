use futures::StreamExt;
use shiplift::{builder::RegistryAuth, Docker, PullOptions};
use std::str;

pub struct DockerHub {
  client: RegistryAuth,
}

impl DockerHub {
  pub fn new() -> DockerHub {
    let client = RegistryAuth::builder().build();

    DockerHub { client: client }
  }

  pub async fn pull_image(&self, image: &str, tag: &str) {
    let docker: Docker = Docker::new();

    let mut stream = docker.images().pull(
      &PullOptions::builder()
        .image(format!("{}:{}", image, tag))
        .auth(self.client.clone())
        .build(),
    );

    while let Some(pull_result) = stream.next().await {
      match pull_result {
        Ok(output) => println!("{:?}", output),
        Err(e) => eprintln!("{}", e),
      };
    }
  }
}
