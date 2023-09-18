use super::events::Event;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InfoPage {
	pub id: String,
	pub event: Event,
	pub title: String,
	pub contents: String,
}
