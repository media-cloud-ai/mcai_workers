#[derive(Debug, Deserialize)]
pub struct License {
  pub key: String,
  pub name: String,
  pub node_id: String,
  pub spdx_id: String,
  pub url: String,
}
