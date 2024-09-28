// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::messages::admin::{
	AdminApplicationData, AdminApplicationUpdate, AdminEntryTypeData, AdminEntryTypeEventData,
	AdminEntryTypeEventUpdate, AdminEntryTypeUpdate, AdminEventData, AdminEventEditorData, AdminEventEditorUpdate,
	AdminEventLogTabsData, AdminEventLogTabsUpdate, AdminEventUpdate, AdminInfoPageData, AdminInfoPageUpdate,
	AdminPermissionGroupData, AdminPermissionGroupUpdate, AdminUserPermissionGroupData, AdminUserPermissionGroupUpdate,
	Application, EditorEventAssociation, EntryTypeEventAssociation, PermissionGroup, PermissionGroupEventAssociation,
	UserPermissionGroupAssociation,
};
use crate::messages::entry_types::EntryType;
use crate::messages::event_log::{EventLogEntry, EventLogTab};
use crate::messages::event_subscription::{EventSubscriptionData, EventSubscriptionUpdate};
use crate::messages::events::Event;
use crate::messages::info_pages::InfoPage;
use crate::messages::permissions::PermissionLevel;
use crate::messages::tags::Tag;
use crate::messages::user::{PublicUserData, SelfUserData, UserSubscriptionUpdate};
use crate::messages::DataError;
use serde::{Deserialize, Serialize};

/// Types of subscriptions to server data
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum SubscriptionType {
	/// A subscription to the event log for a particular event. An event ID is provided with this variant.
	EventLogData(String),
	/// A subscription to all user data.
	AdminUsers,
	/// A subscription to all events.
	AdminEvents,
	/// A subscription to all permission groups and their event associations.
	AdminPermissionGroups,
	/// A subscription to relationships between permission groups and users.
	AdminPermissionGroupUsers,
	/// A subscription to all entry types.
	AdminEntryTypes,
	/// A subscription to relationships between entry types and events.
	AdminEntryTypesEvents,
	/// A subscription to relationships between users (as video editors) and events.
	AdminEventEditors,
	/// A subscription to event log tabs.
	AdminEventLogTabs,
	/// A subscription to all applications.
	AdminApplications,
	/// A subscription to all info pages.
	AdminInfoPages,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InitialEventSubscriptionLoadData {
	/// The event data
	pub event: Event,
	/// The user's permission level for the event
	pub permission: PermissionLevel,
	/// The event entry types that can be used for the event
	pub entry_types: Vec<EntryType>,
	/// The tags that can be used for the event
	pub tags: Vec<Tag>,
	/// The list of users that can be entered as editors
	pub editors: Vec<PublicUserData>,
	/// The list of info pages that can be read for this event
	pub info_pages: Vec<InfoPage>,
	/// The event log tabs
	pub tabs: Vec<EventLogTab>,
	/// The event log entries that have already been created
	pub entries: Vec<EventLogEntry>,
	/// Placeholder data for new entries that haven't yet been created
	pub new_entries: Vec<EventLogEntry>,
}

/// Sent to the client when a new subscription is created.
#[derive(Debug, Deserialize, Serialize)]
pub enum InitialSubscriptionLoadData {
	/// Data for subscribing to an event. Includes the following data:
	/// - The event to which the user subscribed
	/// - The user's permission level for that event
	/// - The event entry types that can be used for that event
	/// - The tags that can be used for that event
	/// - The list of users that can be entered as editors
	/// - The list of info pages that can be read for this event
	/// - The event log section headers
	/// - The event log entries that have already been created
	Event(Box<InitialEventSubscriptionLoadData>),
	AdminUsers(Vec<SelfUserData>),
	AdminEvents(Vec<Event>),
	AdminPermissionGroups(Vec<PermissionGroup>, Vec<PermissionGroupEventAssociation>),
	AdminPermissionGroupUsers(Vec<UserPermissionGroupAssociation>),
	AdminEntryTypes(Vec<EntryType>),
	AdminEntryTypesEvents(Vec<EntryTypeEventAssociation>),
	AdminEventEditors(Vec<EditorEventAssociation>),
	AdminEventLogTabs(Vec<(Event, EventLogTab)>),
	AdminApplications(Vec<Application>),
	AdminInfoPages(Vec<InfoPage>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SubscriptionData {
	EventUpdate(Event, Box<EventSubscriptionData>),
	/// Indicates an update to data related to the logged-in user.
	UserUpdate(UserSubscriptionUpdate),
	AdminEventsUpdate(AdminEventData),
	AdminEntryTypesUpdate(AdminEntryTypeData),
	AdminEntryTypesEventsUpdate(AdminEntryTypeEventData),
	AdminPermissionGroupsUpdate(AdminPermissionGroupData),
	AdminUsersUpdate(SelfUserData),
	AdminEventEditorsUpdate(AdminEventEditorData),
	AdminUserPermissionGroupsUpdate(AdminUserPermissionGroupData),
	AdminEventLogTabsUpdate(AdminEventLogTabsData),
	AdminApplicationsUpdate(AdminApplicationData),
	AdminInfoPagesUpdate(AdminInfoPageData),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SubscriptionFailureInfo {
	NoTarget,
	NotAllowed,
	Error(DataError),
}

/// A client-initiated description detailing for what subscriptions it'd like to send updates
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SubscriptionTargetUpdate {
	EventUpdate(Event, Box<EventSubscriptionUpdate>),
	AdminEventsUpdate(AdminEventUpdate),
	AdminEntryTypesUpdate(AdminEntryTypeUpdate),
	AdminEntryTypesEventsUpdate(AdminEntryTypeEventUpdate),
	AdminPermissionGroupsUpdate(AdminPermissionGroupUpdate),
	AdminUserUpdate(SelfUserData),
	AdminEventEditorsUpdate(AdminEventEditorUpdate),
	AdminUserPermissionGroupsUpdate(AdminUserPermissionGroupUpdate),
	AdminEventLogTabsUpdate(AdminEventLogTabsUpdate),
	AdminApplicationsUpdate(AdminApplicationUpdate),
	AdminInfoPagesUpdate(AdminInfoPageUpdate),
}
