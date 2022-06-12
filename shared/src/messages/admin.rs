use super::events::Event;
use serde::{Deserialize, Serialize};

/// Request for information in admin workflows
#[derive(Deserialize, Serialize)]
pub enum AdminAction {
	AddEvent(NewEvent),
	EditEvent(Event),
	ListEvents,
	ListPermissionGroups,
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

// Response to permission group list containing a list of permission groups
#[derive(Deserialize, Serialize)]
pub struct PermissionGroupList {
	pub permission_groups: Vec<PermissionGroup>,
}

#[derive(Deserialize, Serialize)]
pub struct PermissionGroup {
	pub id: String,
	pub name: String,
}
