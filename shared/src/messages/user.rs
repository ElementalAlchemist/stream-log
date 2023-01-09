use rgb::RGB8;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct UserData {
	pub id: String,
	pub username: String,
	pub is_admin: bool,
	pub color: RGB8,
}
