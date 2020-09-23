#[derive(Debug, Deserialize)]
pub struct Content {
  pub content: String,
  pub download_url: String,
  pub encoding: Encoding,
  pub git_url: String,
  pub html_url: String,
  pub name: String,
  pub path: String,
  pub sha: String,
  pub size: usize,
  #[serde(rename = "type")]
  pub _type: String,
  pub url: String,
  #[serde(rename = "_links")]
  pub links: Links,
}

#[derive(Debug, Deserialize)]
pub enum Encoding {
  #[serde(rename = "base64")]
  Base64,
}

#[derive(Debug, Deserialize)]
pub struct Links {
  pub git: String,
  pub html: String,
  #[serde(rename = "self")]
  pub _self: String,
}
