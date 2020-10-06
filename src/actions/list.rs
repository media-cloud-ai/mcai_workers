use crate::config::McaiWorkersConfig;
use cargo_toml::Manifest;
use clap::ArgMatches;
use colored::Colorize;
use console::Emoji;
use dockerfile_parser::Dockerfile;
use semver::VersionReq;

pub fn list<'a>(cfg: &McaiWorkersConfig, matches: &ArgMatches<'a>) {
  for repo in &cfg.repos {
    println!();
    println!(
      "{} {}",
      Emoji("🚀", &"=>".green().bold()),
      repo.name.green().bold()
    );

    for manifest_content in &repo.manifest_contents {
      let manifest = Manifest::from_str(&manifest_content).unwrap();

      if let Some(package) = &manifest.package {
        println!(
          "  {} {} {}{}",
          Emoji("📙", &"=>".magenta().bold()),
          package.name.yellow(),
          "v".yellow(),
          package.version.yellow()
        );
      }

      if let Some(version) = mcai_worker_sdk_version(&manifest) {
        let extra = cfg
          .mcai_sdk_version
          .as_ref()
          .map(|mcai_sdk_version| {
            VersionReq::parse(&version)
              .map(|version| {
                if !version.matches(&mcai_sdk_version) {
                  Some(format!(
                    "{} Update required to v{}",
                    Emoji("❗", "=>"),
                    mcai_sdk_version
                  ))
                } else {
                  None
                }
              })
              .ok()
              .unwrap_or_default()
          })
          .unwrap_or_default();

        println!(
          "  {} {} {} {}",
          Emoji("📦", &"=>".magenta().bold()),
          "MCAI Worker SDK".magenta(),
          version.magenta(),
          extra.unwrap_or_else(|| "".to_string()).red()
        );
      }
    }

    for dockerfile in &repo.docker_contents {
      let dockerfile = Dockerfile::parse(&dockerfile).unwrap();

      if let Some(image) = docker_information(&dockerfile) {
        println!("  {} {}", Emoji("🐳", &"=>".cyan().bold()), image.cyan());
      }
    }

    if matches.is_present("dependencies") {
      for cargo_content in &repo.manifest_contents {
        let manifest = Manifest::from_str(&cargo_content).unwrap();
        for (name, version) in &manifest.dependencies {
          let version = match version {
            cargo_toml::Dependency::Simple(version) => version.to_string(),
            version => format!("{:?}", version),
          };

          println!("  - {} ({})", name, version);
        }
      }
    }
  }
}

fn mcai_worker_sdk_version(manifest: &Manifest) -> Option<String> {
  for (name, version) in &manifest.dependencies {
    if name == "mcai_worker_sdk" {
      return match version {
        cargo_toml::Dependency::Simple(version) => Some(version.to_string()),
        cargo_toml::Dependency::Detailed(detailed) => {
          if let Some(version) = &detailed.version {
            Some(version.to_string())
          } else {
            Some(detailed.path.clone().unwrap_or_else(|| "".to_string()))
          }
        }
      };
    }
  }
  None
}

fn docker_information(dockerfile: &Dockerfile) -> Option<String> {
  use dockerfile_parser::{ImageRef, Instruction};

  let images = dockerfile
    .instructions
    .iter()
    .filter(|instruction| matches!(instruction, Instruction::From(_)))
    .map(|instruction| {
      if let Instruction::From(content) = instruction {
        content.image_parsed.clone()
      } else {
        unreachable!()
      }
    })
    .collect::<Vec<ImageRef>>();

  for image in &images {
    let reference_images = vec![
      "rust",
      "ubuntu",
      "debian",
      "mediacloudai/rs_command_line_worker",
      "mediacloudai/py_mcai_worker_sdk",
      "mediacloudai/c_mcai_worker_sdk",
    ];

    if !reference_images.contains(&image.image.as_str()) {
      continue;
    }

    let reference_image = match image.image.as_str() {
      "rust" => "Rust",
      "ubuntu" => "Ubuntu",
      "debian" => "Debian",
      "mediacloudai/rs_command_line_worker" => "Command Line",
      "mediacloudai/py_mcai_worker_sdk" => "Python MCAI SDK",
      "mediacloudai/c_mcai_worker_sdk" => "C MCAI SDK",
      _ => unreachable!(),
    };

    return Some(format!(
      "{} {}",
      reference_image,
      image.tag.as_ref().unwrap_or(&"latest".to_string())
    ));
  }
  None
}
