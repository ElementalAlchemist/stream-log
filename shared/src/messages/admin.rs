use super::events::Event;
use serde::{Deserialize, Serialize};

/// Request for information in admin workflows
#[derive(Deserialize, Serialize)]
pub enum AdminAction {
	AddEvent(NewEvent),
	EditEvent(Event),
	ListEvents,
}

/// Information required for adding a new event
#[derive(Deserialize, Serialize)]
pub struct NewEvent {
	pub name: String,
}

/// Response to event list containing a list of events
#[derive(Deserialize, Serialize)]
pub struct EventList {
	pub events: Vec<Event>,
}
