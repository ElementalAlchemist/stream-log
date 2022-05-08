use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct UserData {
	pub id: String,
	pub username: String,
}
