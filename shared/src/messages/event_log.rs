// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::tags::Tag;
use super::user::PublicUserData;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EndTimeData {
	/// Indicates an entered time with the specified accompanying value
	Time(DateTime<Utc>),
	/// Indicates that a time has not yet been entered but will be
	NotEntered,
	/// Indicates that no time is to be entered
	NoTime,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventLogEntry {
	pub id: String,
	pub start_time: DateTime<Utc>,
	pub end_time: EndTimeData,
	pub entry_type: String,
	pub description: String,
	pub media_links: Vec<String>,
	pub submitter_or_winner: String,
	pub tags: Vec<Tag>,
	pub notes_to_editor: String,
	pub editor: Option<PublicUserData>,
	pub video_link: Option<String>,
	pub parent: Option<String>,
	pub created_at: DateTime<Utc>,
	pub manual_sort_key: Option<i32>,
	pub video_processing_state: Option<VideoProcessingState>,
	pub video_errors: String,
	pub poster_moment: bool,
	pub video_edit_state: VideoEditState,
	pub marked_incomplete: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventLogTab {
	pub id: String,
	pub name: String,
	pub start_time: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum VideoProcessingState {
	Unedited,
	Edited,
	Claimed,
	Finalizing,
	Transcoding,
	Done,
	Modified,
	Unlisted,
}

impl VideoProcessingState {
	pub fn all_states() -> Vec<Self> {
		vec![
			Self::Unedited,
			Self::Edited,
			Self::Claimed,
			Self::Finalizing,
			Self::Transcoding,
			Self::Done,
			Self::Modified,
			Self::Unlisted,
		]
	}
}

impl std::fmt::Display for VideoProcessingState {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let str_value = match self {
			Self::Unedited => "UNEDITED",
			Self::Edited => "EDITED",
			Self::Claimed => "CLAIMED",
			Self::Finalizing => "FINALIZING",
			Self::Transcoding => "TRANSCODING",
			Self::Done => "DONE",
			Self::Modified => "MODIFIED",
			Self::Unlisted => "UNLISTED",
		};
		write!(f, "{}", str_value)
	}
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum VideoEditState {
	NoVideo,
	MarkedForEditing,
	DoneEditing,
}

impl VideoEditState {
	pub fn all_states() -> Vec<Self> {
		vec![Self::NoVideo, Self::MarkedForEditing, Self::DoneEditing]
	}
}

impl Default for VideoEditState {
	fn default() -> Self {
		Self::NoVideo
	}
}
