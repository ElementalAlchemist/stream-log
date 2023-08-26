use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tag {
	pub id: String,
	pub name: String,
	pub description: String,
	pub playlist: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TagListUpdate {
	UpdateTag(Tag),
	RemoveTag(Tag),
	ReplaceTag(Tag, Tag),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TagListData {
	UpdateTag(Tag),
	RemoveTag(Tag),
}
