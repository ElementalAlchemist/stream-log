use super::tags::Tag;
use super::user::UserData;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventLogEntry {
	pub id: String,
	pub start_time: DateTime<Utc>,
	pub end_time: Option<DateTime<Utc>>,
	pub entry_type: String,
	pub description: String,
	pub media_link: String,
	pub submitter_or_winner: String,
	pub tags: Vec<Tag>,
	pub notes_to_editor: String,
	pub editor_link: Option<String>,
	pub editor: Option<UserData>,
	pub video_link: Option<String>,
	pub highlighted: bool,
	pub parent: Option<String>,
	pub created_at: DateTime<Utc>,
	pub manual_sort_key: Option<i32>,
	pub video_state: Option<VideoState>,
	pub video_errors: String,
	pub poster_moment: bool,
	pub video_edit_state: VideoEditState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventLogSection {
	pub id: String,
	pub name: String,
	pub start_time: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum VideoState {
	Unedited,
	Edited,
	Claimed,
	Finalizing,
	Transcoding,
	Done,
	Modified,
	Unlisted,
}

impl std::fmt::Display for VideoState {
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum VideoEditState {
	NoVideo,
	MarkedForEditing,
	DoneEditing,
}

impl Default for VideoEditState {
	fn default() -> Self {
		Self::NoVideo
	}
}
