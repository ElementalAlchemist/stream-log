use super::entry_types::EntryType;
use super::event_log::EventLogEntry;
use super::events::Event;
use super::permissions::PermissionLevel;
use super::tags::Tag;
use super::user::UserData;
use super::DataError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum EventSubscriptionResponse {
	Subscribed(Event, PermissionLevel, Vec<EntryType>, Vec<Tag>, Vec<EventLogEntry>),
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
	Typing(TypingData),
	NewTag(Tag),
	DeleteTag(Tag),
	AddEntryType(EntryType),
	DeleteEntryType(EntryType),
}

#[derive(Clone, Deserialize, Serialize)]
pub enum TypingData {
	TypingStartTime(Option<EventLogEntry>, String, UserData),
	TypingEndTime(Option<EventLogEntry>, String, UserData),
	TypingDescription(Option<EventLogEntry>, String, UserData),
	TypingMediaLink(Option<EventLogEntry>, String, UserData),
	TypingSubmitterWinner(Option<EventLogEntry>, String, UserData),
}
