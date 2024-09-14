// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::entry_types::EntryType;
use super::event_log::{EventLogEntry, EventLogTab};
use super::events::Event;
use super::info_pages::InfoPage;
use super::tags::Tag;
use super::user::PublicUserData;
use serde::{Deserialize, Serialize};

/// Event subscription data sent by the server to subscribed clients with information about what changes were made.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum EventSubscriptionData {
	UpdateEvent,
	NewLogEntry(EventLogEntry, PublicUserData),
	DeleteLogEntry(EventLogEntry),
	UpdateLogEntry(EventLogEntry, Option<PublicUserData>),
	Typing(TypingData),
	AddEntryType(EntryType),
	UpdateEntryType(EntryType),
	DeleteEntryType(EntryType),
	AddEditor(PublicUserData),
	RemoveEditor(PublicUserData),
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
	Parent(Option<EventLogEntry>, String, PublicUserData),
	StartTime(Option<EventLogEntry>, String, PublicUserData),
	EndTime(Option<EventLogEntry>, String, PublicUserData),
	EntryType(Option<EventLogEntry>, String, PublicUserData),
	Description(Option<EventLogEntry>, String, PublicUserData),
	MediaLinks(Option<EventLogEntry>, String, PublicUserData),
	SubmitterWinner(Option<EventLogEntry>, String, PublicUserData),
	Notes(Option<EventLogEntry>, String, PublicUserData),
	Clear(Option<EventLogEntry>, PublicUserData),
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
	Notes(Option<EventLogEntry>, String),
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
	Notes,
	Editor,
	MissingGiveawayInfo,
	SortKey,
	Parent,
}
