use super::entry_types::EntryType;
use super::events::Event;
use super::permissions::PermissionLevel;
use super::tags::Tag;
use super::user::UserData;
use serde::{Deserialize, Serialize};

/// Request for information in admin workflows
#[derive(Deserialize, Serialize)]
pub enum AdminAction {
	EditEvents(Vec<Event>),
	ListEvents,
	ListPermissionGroups,
	ListPermissionGroupsWithEvents,
	UpdatePermissionGroups(Vec<PermissionGroupWithEvents>),
	ListUserPermissionGroups(UserData),
	AddUserToPermissionGroup(PermissionGroupUser),
	RemoveUserFromPermissionGroup(PermissionGroupUser),
	ListUsers,
	EditUsers(Vec<UserData>),
	ListUsersWithNoPermissionGroups,
	ListEventTypes,
	AddEventType(EntryType),
	UpdateEventType(EntryType),
	ListEventTypesForEvent(Event),
	UpdateEventTypesForEvent(Event, Vec<EntryType>),
	ListTagsForEvent(Event),
	AddTag(Tag, Event),
	UpdateTagDescription(Tag),
	RemoveTag(Tag),
	ReplaceTag(Tag, Tag),
	CopyTags(Event, Event),
}

/// A single permission group
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct PermissionGroup {
	pub id: String,
	pub name: String,
}

/// A description of an event and its permission level, to be used with a permission group
/// to describe the event's permission level in the group
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventPermission {
	pub event: Event,
	pub level: PermissionLevel,
}

/// List item in response to list permission groups
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
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
