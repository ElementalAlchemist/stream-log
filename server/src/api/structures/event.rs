use serde::Serialize;

/// Event object associated with an event.
#[derive(Serialize)]
pub struct Event {
	/// The event ID to be used for all routes that take an event ID.
	pub id: String,
	/// The event name that can be displayed to users.
	pub name: String,
}
