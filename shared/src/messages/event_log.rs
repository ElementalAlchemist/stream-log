use super::tags::Tag;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct EventLogEntry {
	pub id: String,
	pub start_time: DateTime<Utc>,
	pub end_time: Option<DateTime<Utc>>,
	pub event_type: String,
	pub description: String,
	pub media_link: String,
	pub submitter_or_winner: String,
	pub tags: Vec<Tag>,
	pub notes_to_editor: String,
	pub editor_link: Option<String>,
	pub editor: Option<String>,
	pub highlighted: bool,
}
