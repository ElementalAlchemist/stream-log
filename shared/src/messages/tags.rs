use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Tag {
	pub id: String,
	pub name: String,
	pub description: String,
}
