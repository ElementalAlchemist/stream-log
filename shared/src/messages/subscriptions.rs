use crate::messages::admin::{
	AdminEntryTypeData, AdminEntryTypeEventData, AdminEntryTypeEventUpdate, AdminEntryTypeUpdate, AdminEventData,
	AdminEventEditorUpdate, AdminEventUpdate, AdminPermissionGroupData, AdminPermissionGroupUpdate, AdminTagData,
	AdminTagUpdate, AdminUserPermissionGroupUpdate, EditorEventAssociation, EntryTypeEventAssociation, PermissionGroup,
	PermissionGroupEventAssociation, UserPermissionGroupAssociation,
};
use crate::messages::entry_types::EntryType;
use crate::messages::event_log::EventLogEntry;
use crate::messages::event_subscription::{EventSubscriptionData, EventSubscriptionUpdate};
use crate::messages::events::Event;
use crate::messages::permissions::PermissionLevel;
use crate::messages::tags::Tag;
use crate::messages::user::{UserData, UserSubscriptionUpdate};
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
	/// A subscription to all permission groups.
	AdminPermissionGroups,
	/// A subscription to relationships between permission groups and events.
	AdminPermissionGroupEvents,
	/// A subscription to relationships between permission groups and users.
	AdminPermissionGroupUsers,
	/// A subscription to all entry types.
	AdminEntryTypes,
	/// A subscription to relationships between entry types and events.
	AdminEntryTypesEvents,
	/// A subscription to all tags.
	AdminTags,
	/// A subscription to relationships between users (as video editors) and events.
	AdminEventEditors,
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
	/// - The event log entries that have already been created
	Event(
		Event,
		PermissionLevel,
		Vec<EntryType>,
		Vec<Tag>,
		Vec<UserData>,
		Vec<EventLogEntry>,
	),
	AdminUsers(Vec<UserData>),
	AdminEvents(Vec<Event>),
	AdminPermissionGroups(Vec<PermissionGroup>),
	AdminPermissionGroupEvents(Vec<PermissionGroupEventAssociation>),
	AdminPermissionGroupUsers(Vec<UserPermissionGroupAssociation>),
	AdminEntryTypes(Vec<EntryType>),
	AdminEntryTypesEvents(Vec<EntryTypeEventAssociation>),
	AdminTags(Vec<Tag>),
	AdminEventEditors(Vec<EditorEventAssociation>),
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
	AdminTagsUpdate(AdminTagData),
	AdminUsersUpdate(UserData),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SubscriptionFailureInfo {
	NoTarget,
	NotAllowed,
	Error(DataError),
}

/// A client-initiated description detailing for what subscriptions it'd like to send updates
#[derive(Debug, Deserialize, Serialize)]
pub enum SubscriptionTargetUpdate {
	EventUpdate(Event, Box<EventSubscriptionUpdate>),
	AdminEventsUpdate(AdminEventUpdate),
	AdminEntryTypesUpdate(AdminEntryTypeUpdate),
	AdminEntryTypesEventsUpdate(AdminEntryTypeEventUpdate),
	AdminPermissionGroupsUpdate(AdminPermissionGroupUpdate),
	AdminTagsUpdate(AdminTagUpdate),
	AdminUserUpdate(UserData),
	AdminEventEditorsUpdate(AdminEventEditorUpdate),
	AdminUserPermissionGroupsUpdate(AdminUserPermissionGroupUpdate),
}
