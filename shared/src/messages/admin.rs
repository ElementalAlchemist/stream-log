use super::events::Event;
use super::user::UserData;
use serde::{Deserialize, Serialize};

/// Request for information in admin workflows
#[derive(Deserialize, Serialize)]
pub enum AdminAction {
	MenuInfo,
	UnapprovedUserList,
	ApproveUser(UserData),
	DenyUser(UserData),
	AddEvent(NewEvent),
	EditEvent(Event),
	ListEvents,
}

/// Information required for adding a new event
#[derive(Deserialize, Serialize)]
pub struct NewEvent {
	pub name: String,
}

/// Response for information to show on the menu
#[derive(Deserialize, Serialize)]
pub struct MenuInfo {
	pub unapproved_user_count: u64,
}

/// Response for list of unapprofed users
#[derive(Deserialize, Serialize)]
pub struct UnapprovedUsers {
	pub users: Vec<UserData>,
}

/// Response to event list containing a list of events
#[derive(Deserialize, Serialize)]
pub struct EventList {
	pub events: Vec<Event>,
}
