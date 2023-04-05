use super::entry_types::EntryType;
use super::events::Event;
use super::permissions::PermissionLevel;
use super::tags::Tag;
use super::user::UserData;
use serde::{Deserialize, Serialize};

/// An update to an event from the admin events page
#[derive(Debug, Deserialize, Serialize)]
pub enum AdminEventUpdate {
	UpdateEvent(Event),
}

/// An update to an entry type from the admin entry types page
#[derive(Debug, Deserialize, Serialize)]
pub enum AdminEntryTypeUpdate {
	UpdateEntryType(EntryType),
}

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

#[derive(Debug, Deserialize, Serialize)]
pub enum AdminEventEditorUpdate {
	AddEditor(EditorEventAssociation),
	RemoveEditor(EditorEventAssociation),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EditorEventAssociation {
	pub editor: UserData,
	pub event: Event,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum AdminUserPermissionGroupUpdate {
	AddUserToGroup(UserPermissionGroupAssociation),
	RemoveUserFromGroup(UserPermissionGroupAssociation),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserPermissionGroupAssociation {
	pub user: UserData,
	pub permission_group: PermissionGroup,
}
