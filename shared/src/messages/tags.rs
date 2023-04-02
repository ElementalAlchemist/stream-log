use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tag {
	pub id: String,
	pub name: String,
	pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagEventAssociation {
	pub tag: String,
	pub event: String,
}
