use rgb::RGB8;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct EventType {
	pub id: String,
	pub name: String,
	pub color: RGB8,
}
