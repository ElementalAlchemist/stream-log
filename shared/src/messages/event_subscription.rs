use super::event_log::EventLogEntry;
use super::event_types::EventType;
use super::events::Event;
use super::permissions::PermissionLevel;
use super::tags::Tag;
use super::user::UserData;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum EventSubscriptionResponse {
	Subscribed(Event, PermissionLevel, Vec<EventType>, Vec<Tag>, Vec<EventLogEntry>),
	NoEvent,
	NotAllowed,
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
	AddEventType(EventType),
	DeleteEventType(EventType),
}

#[derive(Clone, Deserialize, Serialize)]
pub enum TypingData {
	TypingStartTime(Option<EventLogEntry>, String, UserData),
	TypingEndTime(Option<EventLogEntry>, String, UserData),
	TypingDescription(Option<EventLogEntry>, String, UserData),
	TypingMediaLink(Option<EventLogEntry>, String, UserData),
	TypingSubmitterWinner(Option<EventLogEntry>, String, UserData),
}
