use super::events::Event;
use super::user::UserData;
use serde::{Deserialize, Serialize};

/// Request for information in admin workflows
#[derive(Deserialize, Serialize)]
pub enum AdminAction {
	AddEvent(NewEvent),
	EditEvent(Event),
	ListEvents,
	ListPermissionGroups,
	CreatePermissionGroup(String),
	SetEventViewForGroup(PermissionGroupEvent),
	SetEventEditForGroup(PermissionGroupEvent),
	RemoveEventFromGroup(PermissionGroupEvent),
	AddUserToPermissionGroup(PermissionGroupUser),
	RemoveUserFromPermissionGroup(PermissionGroupUser),
	ListUsers,
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

/// A single permission group
#[derive(Deserialize, Serialize)]
pub struct PermissionGroup {
	pub id: String,
	pub name: String,
}

/// A pairing of a permission group and its associated event
#[derive(Deserialize, Serialize)]
pub struct PermissionGroupEvent {
	pub group: PermissionGroup,
	pub event: Event,
}

/// A pairing of a permission group and a user
#[derive(Deserialize, Serialize)]
pub struct PermissionGroupUser {
	pub group: PermissionGroup,
	pub user: UserData,
}

/// List item in response to list users
#[derive(Deserialize, Serialize)]
pub struct UserDataPermissions {
	pub user: UserData,
	pub groups: Vec<PermissionGroup>,
}
