use crate::{
  config::{McaiWorkersConfig, Provider, Registry, RepoConfig},
  dockerhub::DockerHub,
  github::Github,
  gitlab::Gitlab,
};
use clap::value_t;
use clap::ArgMatches;
use dockerfile_parser::Dockerfile;
use futures::StreamExt;
use mcai_worker_sdk::job::Job;
use shiplift::{
  rep::ContainerCreateInfo, tty::TtyChunk, Container, ContainerOptions, Docker, LogsOptions,
};
use std::collections::HashMap;
use std::env::temp_dir;
use std::fs::File;
use std::str;
use uuid::Uuid;

#[tokio::main]
pub async fn benchmark<'a>(cfg: &mut McaiWorkersConfig, matches: &ArgMatches<'a>) {
  let worker = matches.value_of("worker").unwrap();
  if let Some(worker_repo) = &cfg
    .repos
    .iter()
    .find(|repo| repo.image.as_str().contains(worker))
  {
    let job = get_example_job(&worker_repo, &matches).unwrap();

    let command = parse_command(worker_repo.docker_contents.first().unwrap());
    let cpus = value_t!(matches.value_of("cpus"), f64).unwrap();
    let memory = value_t!(matches.value_of("memory"), u64).unwrap();
    let tag = matches.value_of("tag").unwrap();

    let path = create_temporary_file(&job);
    let envs = parse_envs(&job, &matches.value_of("envs").unwrap().to_string());

    let registry = match worker_repo.registry {
      Registry::DockerHub => DockerHub::new(),
      _ => panic!("unknown registry"),
    };

    registry.pull_image(worker_repo.image.as_str(), tag).await;

    let container_info = create_container(
      &worker_repo.image,
      &tag.to_string(),
      &command,
      &envs,
      &path,
      memory,
      cpus,
    )
    .await;

    execute_container(&container_info.id).await;
  } else {
    println!(
      "Worker {} is not registered, do 'mcai-workers register --help' to get more information.",
      worker
    );
  }
}

fn create_temporary_file(job: &Job) -> String {
  let path = format!("{}/{}.json", temp_dir().to_str().unwrap(), Uuid::new_v4());
  serde_json::to_writer(&File::create(&path).unwrap(), &job).unwrap();
  path
}

async fn create_container(
  image: &String,
  tag: &String,
  commands: &Vec<String>,
  envs: &HashMap<String, String>,
  example_path: &String,
  memory: u64,
  cpus: f64,
) -> ContainerCreateInfo {
  let docker = Docker::new();

  let cmd = commands.iter().map(AsRef::as_ref).collect();
  let mut environment_variables = vec![
    "SOURCE_ORDERS=/examples/job.json".to_string(),
    "RUST_LOG=debug".to_string(),
  ];
  let mut volumes = vec![format!("{}:/examples/job.json", example_path)];
  for (key, val) in envs.iter() {
    environment_variables.push(format!("{}={}", key, val));
    volumes.push(format!("{}:{}", val, val));
  }

  let options = ContainerOptions::builder(format!("{}:{}", image, tag).as_str())
    .cmd(cmd)
    .volumes(volumes.iter().map(AsRef::as_ref).collect())
    .env(environment_variables)
    .cpus(cpus)
    .memory(memory)
    .attach_stdout(true)
    .attach_stderr(true)
    .build();

  match docker.containers().create(&options).await {
    Ok(info) => info,
    Err(error) => panic!(format!("Could not create container: {:?}", error)),
  }
}

async fn execute_container(id: &String) {
  let docker = Docker::new();
  let container = Container::new(&docker, id);
  let log_options = LogsOptions::builder().stdout(true).stderr(true).tail("all").build();

  match container.start().await {
    Ok(_) => {
      while let Some(exec_result) = container.logs(&log_options).next().await {
        match exec_result {
          Ok(chunk) => print_chunk(chunk),
          Err(e) => eprintln!("Error: {}", e),
        }
      }
    }
    Err(error) => println!("Impossible to start container: {}", error),
  }
}

fn get_example_job<'a>(repo: &RepoConfig, matches: &ArgMatches<'a>) -> Option<Job> {
  let example = matches.value_of("example").unwrap();
  if let Some(example_filename) = &repo
    .example_filenames
    .iter()
    .find(|file| file.as_str().contains(example))
  {
    if let Some(content) = match repo.provider {
      Provider::Github => {
        let token = matches.value_of("github-token").unwrap();
        let github = Github::new(token);
        github.get_file_content(repo.name.as_str(), example_filename)
      }
      Provider::Gitlab => {
        let token = matches.value_of("gitlab-token").unwrap();
        let gitlab = Gitlab::new(token);
        gitlab.get_file_content(repo.name.as_str(), example_filename)
      }
    } {
      let job: Job = serde_json::from_str(content.as_str()).unwrap();

      Some(job)
    } else {
      None
    }
  } else {
    None
  }
}

fn parse_command(content: &str) -> Vec<String> {
  use dockerfile_parser::{BreakableStringComponent, CmdInstruction, Instruction};

  let dockerfile: Dockerfile = Dockerfile::parse(content).unwrap();

  if let Some(Instruction::Cmd(instruction)) = dockerfile
    .instructions
    .iter()
    .find(|instruction| matches!(instruction, Instruction::Cmd(_)))
  {
    match instruction {
      CmdInstruction::Exec(commands) => commands.clone(),
      CmdInstruction::Shell(breakable_string) => breakable_string
        .components
        .iter()
        .map(|item| {
          if let BreakableStringComponent::String(command) = item {
            command.content.clone()
          } else {
            unreachable!()
          }
        })
        .collect::<Vec<String>>(),
    }
  } else {
    unreachable!()
  }
}

fn parse_envs(job: &Job, value: &String) -> HashMap<String, String> {
  let envs: HashMap<String, String> = serde_json::from_str(value.as_str()).unwrap();

  job
    .parameters
    .iter()
    .filter(|parameter| {
      parameter
        .store
        .clone()
        .unwrap_or_else(|| "value".to_string())
        .contains("environment")
    })
    .map(|parameter| {
      let name = parameter.value.clone().unwrap();
      (
        name.as_str().unwrap().to_string(),
        envs
          .get(name.as_str().unwrap())
          .unwrap_or_else(|| panic!("Env '{}' must be defined.", name))
          .clone(),
      )
    })
    .collect::<HashMap<String, String>>()
}

fn print_chunk(chunk: TtyChunk) {
  match chunk {
    TtyChunk::StdOut(bytes) => println!("Stdout: {}", str::from_utf8(&bytes).unwrap()),
    TtyChunk::StdErr(bytes) => eprintln!("Stdout: {}", str::from_utf8(&bytes).unwrap()),
    TtyChunk::StdIn(_) => unreachable!(),
  }
}
