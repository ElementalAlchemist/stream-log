use super::events::Event;
use super::permissions::PermissionLevel;
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

/// A single permission group
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

/// A description of an event and its permission level, to be used with a permission group
/// to describe the event's permission level in the group
#[derive(Clone, Deserialize, Serialize)]
pub struct EventPermission {
	pub event: Event,
	pub level: PermissionLevel,
}

/// List item in response to list permission groups
#[derive(Clone, Deserialize, Serialize)]
pub struct PermissionGroupWithEvents {
	pub group: PermissionGroup,
	pub events: Vec<EventPermission>,
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
