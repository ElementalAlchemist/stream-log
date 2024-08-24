// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::entry_types::EntryType;
use super::event_log::EventLogTab;
use super::events::Event;
use super::info_pages::InfoPage;
use super::permissions::PermissionLevel;
use super::user::PublicUserData;
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
	pub editor: PublicUserData,
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
	pub user: PublicUserData,
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
pub enum AdminEventLogTabsData {
	AddTab(Event, EventLogTab),
	UpdateTab(EventLogTab),
	DeleteTab(EventLogTab),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AdminEventLogTabsUpdate {
	AddTab(Event, EventLogTab),
	UpdateTab(EventLogTab),
	DeleteTab(EventLogTab),
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
