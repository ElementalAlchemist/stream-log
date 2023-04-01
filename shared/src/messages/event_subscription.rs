use super::entry_types::EntryType;
use super::event_log::EventLogEntry;
use super::events::Event;
use super::tags::Tag;
use super::user::UserData;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Event subscription data sent by the server to subscribed clients with information about what changes were made.
#[derive(Clone, Debug, Deserialize, Serialize)]
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

/// Typing data sent by the server as part of event subscription data with information on what updates to make to typing
/// data by other users.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TypingData {
	StartTime(Option<EventLogEntry>, String, UserData),
	EndTime(Option<EventLogEntry>, String, UserData),
	EntryType(Option<EventLogEntry>, String, UserData),
	Description(Option<EventLogEntry>, String, UserData),
	MediaLink(Option<EventLogEntry>, String, UserData),
	SubmitterWinner(Option<EventLogEntry>, String, UserData),
	NotesToEditor(Option<EventLogEntry>, String, UserData),
}

/// Event subscription update sent by the client to the server.
#[derive(Debug, Deserialize, Serialize)]
pub enum EventSubscriptionUpdate {
	NewLogEntry(EventLogEntry),
	DeleteLogEntry(EventLogEntry),
	ChangeStartTime(EventLogEntry, DateTime<Utc>),
	ChangeEndTime(EventLogEntry, Option<DateTime<Utc>>),
	/// Updates the entry type for the given [`EventLogEntry`]. Accepts a string ID.
	ChangeEntryType(EventLogEntry, String),
	ChangeDescription(EventLogEntry, String),
	ChangeMediaLink(EventLogEntry, String),
	ChangeSubmitterWinner(EventLogEntry, String),
	ChangeTags(EventLogEntry, Vec<Tag>),
	ChangeMakeVideo(EventLogEntry, bool),
	ChangeNotesToEditor(EventLogEntry, String),
	ChangeEditor(EventLogEntry, Option<UserData>),
	ChangeHighlighted(EventLogEntry, bool),
	Typing(NewTypingData),
	NewTag(Tag),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum NewTypingData {
	StartTime(Option<EventLogEntry>, String),
	EndTime(Option<EventLogEntry>, String),
	EntryType(Option<EventLogEntry>, String),
	Description(Option<EventLogEntry>, String),
	MediaLink(Option<EventLogEntry>, String),
	SubmitterWinner(Option<EventLogEntry>, String),
	NotesToEditor(Option<EventLogEntry>, String),
}
