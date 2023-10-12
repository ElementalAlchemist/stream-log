use super::entry_types::EntryType;
use super::event_log::{EndTimeData, EventLogEntry, EventLogSection, VideoEditState};
use super::events::Event;
use super::info_pages::InfoPage;
use super::tags::Tag;
use super::user::UserData;
use chrono::{DateTime, Utc};
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
	UpdateSection(EventLogSection),
	DeleteSection(EventLogSection),
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
	ChangeStartTime(EventLogEntry, DateTime<Utc>),
	ChangeEndTime(EventLogEntry, EndTimeData),
	/// Updates the entry type for the given [`EventLogEntry`]. Accepts a string ID.
	ChangeEntryType(EventLogEntry, String),
	ChangeDescription(EventLogEntry, String),
	ChangeMediaLinks(EventLogEntry, Vec<String>),
	ChangeSubmitterWinner(EventLogEntry, String),
	ChangePosterMoment(EventLogEntry, bool),
	ChangeTags(EventLogEntry, Vec<Tag>),
	ChangeVideoEditState(EventLogEntry, VideoEditState),
	ChangeNotesToEditor(EventLogEntry, String),
	ChangeEditor(EventLogEntry, Option<UserData>),
	ChangeIsIncomplete(EventLogEntry, bool),
	ChangeManualSortKey(EventLogEntry, Option<i32>),
	ChangeParent(EventLogEntry, Option<Box<EventLogEntry>>),
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
