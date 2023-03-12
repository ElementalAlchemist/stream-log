use super::entry_types::EntryType;
use super::event_log::EventLogEntry;
use super::events::Event;
use super::permissions::PermissionLevel;
use super::tags::Tag;
use super::user::UserData;
use super::DataError;
use serde::{Deserialize, Serialize};

/// A response to an initial subscription request. Responds with data about the subscription or why the subscription was
/// unsuccessful.
#[derive(Deserialize, Serialize)]
pub enum EventSubscriptionResponse {
	/// The response used when the subscription was successful. Responds with the following data:
	/// - The event to which the user subscribed
	/// - The user's permission level for that event
	/// - The event entry types that can be used for that event
	/// - The tags that can be used for that event
	/// - The list of users that can be entered as editors
	/// - The event log entries that have already been created
	Subscribed(
		Event,
		PermissionLevel,
		Vec<EntryType>,
		Vec<Tag>,
		Vec<UserData>,
		Vec<EventLogEntry>,
	),
	NoEvent,
	NotAllowed,
	Error(DataError),
}

#[derive(Deserialize, Serialize)]
pub enum EventUnsubscriptionResponse {
	Success,
}

#[derive(Clone, Deserialize, Serialize)]
pub enum EventSubscriptionData {
	NewLogEntry(EventLogEntry),
	DeleteLogEntry(EventLogEntry),
	UpdateLogEntry(EventLogEntry),
	Typing(Event, TypingData),
	NewTag(Event, Tag),
	DeleteTag(Event, Tag),
	AddEntryType(Event, EntryType),
	DeleteEntryType(Event, EntryType),
	AddEditor(Event, UserData),
	RemoveEditor(Event, UserData),
}

#[derive(Clone, Deserialize, Serialize)]
pub enum TypingData {
	TypingStartTime(Option<EventLogEntry>, String, UserData),
	TypingEndTime(Option<EventLogEntry>, String, UserData),
	TypingDescription(Option<EventLogEntry>, String, UserData),
	TypingMediaLink(Option<EventLogEntry>, String, UserData),
	TypingSubmitterWinner(Option<EventLogEntry>, String, UserData),
	ClearTyping(Option<EventLogEntry>, UserData),
}
