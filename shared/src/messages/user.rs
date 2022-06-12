use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct UserData {
	pub id: String,
	pub username: String,
	pub is_admin: bool,
}
