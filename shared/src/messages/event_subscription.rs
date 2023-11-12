use super::entry_types::EntryType;
use super::event_log::{EventLogEntry, EventLogTab};
use super::events::Event;
use super::info_pages::InfoPage;
use super::tags::Tag;
use super::user::UserData;
use serde::{Deserialize, Serialize};

/// Event subscription data sent by the server to subscribed clients with information about what changes were made.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum EventSubscriptionData {
	UpdateEvent,
	NewLogEntry(EventLogEntry, UserData),
	DeleteLogEntry(EventLogEntry),
	UpdateLogEntry(EventLogEntry, Option<UserData>),
	Typing(TypingData),
	AddEntryType(EntryType),
	UpdateEntryType(EntryType),
	DeleteEntryType(EntryType),
	AddEditor(UserData),
	RemoveEditor(UserData),
	UpdateInfoPage(InfoPage),
	DeleteInfoPage(InfoPage),
	UpdateTab(EventLogTab),
	DeleteTab(EventLogTab),
	UpdateTag(Tag),
	RemoveTag(Tag),
}

/// Typing data sent by the server as part of event subscription data with information on what updates to make to typing
/// data by other users.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TypingData {
	Parent(Option<EventLogEntry>, String, UserData),
	StartTime(Option<EventLogEntry>, String, UserData),
	EndTime(Option<EventLogEntry>, String, UserData),
	EntryType(Option<EventLogEntry>, String, UserData),
	Description(Option<EventLogEntry>, String, UserData),
	MediaLinks(Option<EventLogEntry>, String, UserData),
	SubmitterWinner(Option<EventLogEntry>, String, UserData),
	NotesToEditor(Option<EventLogEntry>, String, UserData),
	Clear(Option<EventLogEntry>, UserData),
}

/// Event subscription update sent by the client to the server.
#[derive(Debug, Deserialize, Serialize)]
pub enum EventSubscriptionUpdate {
	NewLogEntry(EventLogEntry, u8),
	DeleteLogEntry(EventLogEntry),
	UpdateLogEntry(EventLogEntry, Vec<ModifiedEventLogEntryParts>),
	Typing(NewTypingData),
	UpdateTag(Tag),
	RemoveTag(Tag),
	ReplaceTag(Tag, Tag),
	CopyTagsFromEvent(Event),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum NewTypingData {
	Parent(Option<EventLogEntry>, String),
	StartTime(Option<EventLogEntry>, String),
	EndTime(Option<EventLogEntry>, String),
	EntryType(Option<EventLogEntry>, String),
	Description(Option<EventLogEntry>, String),
	MediaLinks(Option<EventLogEntry>, String),
	SubmitterWinner(Option<EventLogEntry>, String),
	NotesToEditor(Option<EventLogEntry>, String),
	Clear(Option<EventLogEntry>),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ModifiedEventLogEntryParts {
	StartTime,
	EndTime,
	EntryType,
	Description,
	MediaLinks,
	SubmitterOrWinner,
	Tags,
	VideoEditState,
	PosterMoment,
	NotesToEditor,
	Editor,
	MarkedIncomplete,
	SortKey,
	Parent,
}
