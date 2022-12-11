use rgb::RGB8;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventType {
	pub id: String,
	pub name: String,
	pub color: RGB8,
}
