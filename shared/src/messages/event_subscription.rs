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
	UpdateLogEntry(EventLogEntry, Option<PublicUserData>),
	DeleteLogEntry(EventLogEntry),
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
	Parent(EventLogEntry, String, PublicUserData),
	StartTime(EventLogEntry, String, PublicUserData),
	EndTime(EventLogEntry, String, PublicUserData),
	EntryType(EventLogEntry, String, PublicUserData),
	Description(EventLogEntry, String, PublicUserData),
	MediaLinks(EventLogEntry, String, PublicUserData),
	SubmitterWinner(EventLogEntry, String, PublicUserData),
	Notes(EventLogEntry, String, PublicUserData),
	Clear(EventLogEntry, PublicUserData),
}

/// Event subscription update sent by the client to the server.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum EventSubscriptionUpdate {
	UpdateLogEntry(EventLogEntry, Vec<ModifiedEventLogEntryParts>),
	DeleteLogEntry(EventLogEntry),
	Typing(NewTypingData),
	UpdateTag(Tag),
	RemoveTag(Tag),
	ReplaceTag(Tag, Tag),
	CopyTagsFromEvent(Event),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum NewTypingData {
	Parent(EventLogEntry, String),
	StartTime(EventLogEntry, String),
	EndTime(EventLogEntry, String),
	EntryType(EventLogEntry, String),
	Description(EventLogEntry, String),
	MediaLinks(EventLogEntry, String),
	SubmitterWinner(EventLogEntry, String),
	Notes(EventLogEntry, String),
	Clear(EventLogEntry),
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
