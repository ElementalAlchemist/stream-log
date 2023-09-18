use super::entry_types::EntryType;
use super::event_log::EventLogSection;
use super::events::Event;
use super::info_pages::InfoPage;
use super::permissions::PermissionLevel;
use super::user::UserData;
use serde::{Deserialize, Serialize};

/// An update to an event from the admin events page
#[derive(Debug, Deserialize, Serialize)]
pub enum AdminEventUpdate {
	UpdateEvent(Event),
}

/// Data for a server-processed change for the admin events page
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminEventData {
	UpdateEvent(Event),
}

/// An update to an entry type from the admin entry types page
#[derive(Debug, Deserialize, Serialize)]
pub enum AdminEntryTypeUpdate {
	UpdateEntryType(EntryType),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminEntryTypeData {
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
	UpdateGroup(PermissionGroup),
	SetEventPermissionForGroup(PermissionGroupEventAssociation),
	RemoveEventFromGroup(PermissionGroup, Event),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminPermissionGroupData {
	UpdateGroup(PermissionGroup),
	SetEventPermissionForGroup(PermissionGroupEventAssociation),
	RemoveEventFromGroup(PermissionGroup, Event),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum AdminEventEditorUpdate {
	AddEditor(EditorEventAssociation),
	RemoveEditor(EditorEventAssociation),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EditorEventAssociation {
	pub editor: UserData,
	pub event: Event,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminEventEditorData {
	AddEditor(EditorEventAssociation),
	RemoveEditor(EditorEventAssociation),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum AdminUserPermissionGroupUpdate {
	AddUserToGroup(UserPermissionGroupAssociation),
	RemoveUserFromGroup(UserPermissionGroupAssociation),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserPermissionGroupAssociation {
	pub user: UserData,
	pub permission_group: PermissionGroup,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminUserPermissionGroupData {
	AddUserToGroup(UserPermissionGroupAssociation),
	RemoveUserFromGroup(UserPermissionGroupAssociation),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum AdminEntryTypeEventUpdate {
	AddTypeToEvent(EntryTypeEventAssociation),
	RemoveTypeFromEvent(EntryTypeEventAssociation),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminEntryTypeEventData {
	AddTypeToEvent(EntryTypeEventAssociation),
	RemoveTypeFromEvent(EntryTypeEventAssociation),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntryTypeEventAssociation {
	pub entry_type: EntryType,
	pub event: Event,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminEventLogSectionsData {
	AddSection(Event, EventLogSection),
	UpdateSection(EventLogSection),
	DeleteSection(EventLogSection),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminEventLogSectionsUpdate {
	AddSection(Event, EventLogSection),
	UpdateSection(EventLogSection),
	DeleteSection(EventLogSection),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Application {
	pub id: String,
	pub name: String,
	pub read_log: bool,
	pub write_links: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminApplicationData {
	UpdateApplication(Application),
	ShowApplicationAuthKey(Application, String),
	RevokeApplication(Application),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminApplicationUpdate {
	UpdateApplication(Application),
	ResetAuthToken(Application),
	RevokeApplication(Application),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminInfoPageData {
	UpdateInfoPage(InfoPage),
	DeleteInfoPage(InfoPage),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminInfoPageUpdate {
	UpdateInfoPage(InfoPage),
	DeleteInfoPage(InfoPage),
}
