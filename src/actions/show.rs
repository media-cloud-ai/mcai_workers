use crate::config::McaiWorkersConfig;
use colored::Colorize;

pub fn show(cfg: &McaiWorkersConfig) {
  for repo in &cfg.repos {
    let provider = format!("{:?}", repo.provider);
    println!("{} {}", provider.green().bold(), repo.name);
  }
}
