use rgb::RGB8;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EntryType {
	pub id: String,
	pub name: String,
	pub description: String,
	pub color: RGB8,
	pub require_end_time: bool,
}
