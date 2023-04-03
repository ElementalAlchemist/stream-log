use super::events::Event;
use super::permissions::PermissionLevel;
use super::tags::Tag;
use super::user::UserData;
use serde::{Deserialize, Serialize};

/// A single permission group
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct PermissionGroup {
	pub id: String,
	pub name: String,
}

/// An association of a permission group and its relevant event permissions
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PermissionGroupEventAssociation {
	pub group: String,
	pub event: String,
	pub permission: PermissionLevel,
}

/// A pairing of a permission group and a user
#[derive(Deserialize, Serialize)]
pub struct PermissionGroupUser {
	pub group: PermissionGroup,
	pub user: UserData,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum AdminPermissionGroupUpdate {
	AddGroup(PermissionGroup),
	UpdateGroup(PermissionGroup),
	SetEventPermissionForGroup(PermissionGroupEventAssociation),
	RemoveEventFromGroup(PermissionGroup, Event),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum AdminTagUpdate {
	UpdateTag(Tag),
	AddTag(Tag, Event),
	RemoveTag(Tag),
	ReplaceTag(Tag, Tag),
}
