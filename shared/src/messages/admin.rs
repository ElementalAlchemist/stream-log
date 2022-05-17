use super::events::Event;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum AdminAction {
	AddEvent(NewEvent),
	EditEvent(Event),
	ListEvents,
}

#[derive(Deserialize, Serialize)]
pub struct NewEvent {
	pub name: String,
}

#[derive(Deserialize, Serialize)]
pub struct EventList {
	pub events: Vec<Event>,
}
