use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tag {
	pub id: String,
	pub name: String,
	pub description: String,
	pub playlist: String,
}
