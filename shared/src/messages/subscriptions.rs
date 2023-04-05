use crate::messages::admin::{
	AdminEntryTypeUpdate, AdminEventEditorUpdate, AdminEventUpdate, AdminPermissionGroupUpdate, AdminTagUpdate,
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

#[derive(Debug, Deserialize, Serialize)]
pub enum SubscriptionType {
	EventLogData(String),
	AdminUsers,
	AdminEvents,
	AdminPermissionGroups,
	AdminPermissionGroupUsers,
	AdminEntryTypes,
	AdminEntryTypesEvents,
	AdminTags,
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
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SubscriptionData {
	EventUpdate(Event, Box<EventSubscriptionData>),
	/// Indicates an update to data related to the logged-in user.
	UserUpdate(UserSubscriptionUpdate),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SubscriptionFailureInfo {
	NoTarget,
	NotAllowed,
	Error(DataError),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum SubscriptionTargetUpdate {
	EventUpdate(Event, Box<EventSubscriptionUpdate>),
	AdminEventsUpdate(AdminEventUpdate),
	AdminEntryTypesUpdate(AdminEntryTypeUpdate),
	AdminPermissionGroupsUpdate(AdminPermissionGroupUpdate),
	AdminTagsUpdate(AdminTagUpdate),
	AdminUserUpdate(UserData),
	AdminEventEditorsUpdate(AdminEventEditorUpdate),
}
