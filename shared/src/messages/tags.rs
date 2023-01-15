use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tag {
	pub id: String,
	pub name: String,
	pub description: String,
}
