#[derive(Debug, Deserialize)]
pub struct Permissions {
  pub admin: bool,
  pub pull: bool,
  pub push: bool,
}
