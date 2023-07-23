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
	pub make_video: bool,
	pub notes_to_editor: String,
	pub editor_link: Option<String>,
	pub editor: Option<UserData>,
	pub video_link: Option<String>,
	pub highlighted: bool,
	pub parent: Option<String>,
	pub created_at: DateTime<Utc>,
	pub manual_sort_key: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EventLogSection {
	pub id: String,
	pub name: String,
	pub start_time: DateTime<Utc>,
}
