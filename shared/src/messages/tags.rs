use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tag {
	pub id: String,
	pub name: String,
	pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AvailableTagData {
	UpdateTag(Tag),
	RemoveTag(Tag),
}
