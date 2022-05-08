use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct EventSelection {
	pub available_events: Vec<Event>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Event {
	pub id: String,
	pub name: String,
}
