use super::entry_type::EntryType;
use super::user::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The event log entry object representing an entry in the event log.
#[derive(Deserialize, Serialize)]
pub struct EventLogEntry {
	/// The ID of the entry
	pub id: String,
	/// The start time of the entry
	pub start_time: DateTime<Utc>,
	/// The end time of the entry, if entered
	pub end_time: Option<DateTime<Utc>>,
	/// The entry type this entry has
	pub entry_type: EntryType,
	/// The entry description
	pub description: String,
	/// The media link associated with the entry
	pub media_link: String,
	/// The submitter or winner related to the entry
	pub submitter_or_winner: String,
	/// The notes about this entry to the editor
	pub notes_to_editor: String,
	/// The link to the video editor page for this entry
	pub editor_link: Option<String>,
	/// The editor assigned to this entry
	pub editor: Option<User>,
	/// The link to the uploaded video for this entry
	pub video_link: Option<String>,
	/// The ID of the parent entry, if this entry is a child
	pub parent: Option<String>,
	/// The entered manual sort key for the entry
	pub manual_sort_key: Option<i32>,
	/// Whether this entry is marked as a poster moment
	pub poster_moment: bool,
	/// Whether this entry is marked as incomplete
	pub marked_incomplete: bool,
}
