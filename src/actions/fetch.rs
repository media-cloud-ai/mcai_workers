use crate::{
  config::{McaiWorkersConfig, Provider},
  github::Github,
  gitlab::Gitlab,
};
use clap::ArgMatches;

pub fn fetch<'a>(cfg: &mut McaiWorkersConfig, matches: &ArgMatches<'a>) {
  for repo in cfg.repos.iter_mut() {
    println!("Fetch {}", repo.name);
    match repo.provider {
      Provider::Github => {
        let token = matches.value_of("github-token").unwrap();
        let github = Github::new(token);

        let manifest_filenames = if repo.manifest_filenames.is_empty() {
          vec!["Cargo.toml".to_string()]
        } else {
          repo.manifest_filenames.clone()
        };

        repo.manifest_contents.clear();
        for manifest_filename in manifest_filenames {
          if let Some(cargo_toml_manifest) = github.get_file_content(&repo.name, &manifest_filename)
          {
            repo.manifest_contents.push(cargo_toml_manifest);
          }
        }

        let docker_filenames = if repo.docker_filenames.is_empty() {
          vec!["Dockerfile".to_string()]
        } else {
          repo.docker_filenames.clone()
        };

        repo.docker_contents.clear();
        for docker_filename in docker_filenames {
          if let Some(dockerfile) = github.get_file_content(&repo.name, &docker_filename) {
            repo.docker_contents.push(dockerfile);
          }
        }
      }
      Provider::Gitlab => {
        let token = matches.value_of("gitlab-token").unwrap();
        let gitlab = Gitlab::new(token);

        if let Some(cargo_toml_manifest) = gitlab.get_file_content(&repo.name, "Cargo.toml") {
          repo.manifest_contents.push(cargo_toml_manifest);
        }

        repo.docker_contents.clear();
        if let Some(dockerfile) = gitlab.get_file_content(&repo.name, "Dockerfile") {
          repo.docker_contents.push(dockerfile);
        }
      }
    }
  }

  cfg.store();
}
