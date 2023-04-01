use rgb::RGB8;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EntryType {
	pub id: String,
	pub name: String,
	pub color: RGB8,
}
