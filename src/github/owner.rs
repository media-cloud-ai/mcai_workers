#[derive(Debug, Deserialize)]
pub struct Owner {
  pub avatar_url: String,
  pub events_url: String,
  pub followers_url: String,
  pub following_url: String,
  pub gists_url: String,
  pub gravatar_id: String,
  pub html_url: String,
  pub id: usize,
  pub login: String,
  pub node_id: String,
  pub organizations_url: String,
  pub received_events_url: String,
  pub repos_url: String,
  pub site_admin: bool,
  pub starred_url: String,
  pub subscriptions_url: String,
  #[serde(rename = "type")]
  pub _type: String,
  pub url: String,
}
