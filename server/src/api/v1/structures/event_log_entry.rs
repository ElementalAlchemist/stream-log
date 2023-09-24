use super::entry_type::EntryType;
use super::tag::Tag;
use super::user::User;
use super::video_edit_state::VideoEditState;
use super::video_state::VideoState;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// The event log entry object representing an entry in the event log.
#[derive(Serialize)]
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
	pub media_links: Vec<String>,
	/// The submitter or winner related to the entry
	pub submitter_or_winner: String,
	/// The tags associated with the entry
	pub tags: Vec<Tag>,
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
	/// The currently selected edit state for the video. This state is determined by user entry.
	pub video_edit_state: VideoEditState,
	/// The current state of the video for the entry, if set
	pub video_state: Option<VideoState>,
	/// Video errors for this entry; if empty, no video errors are set for this entry
	pub video_errors: String,
	/// Whether this entry is marked as a poster moment
	pub poster_moment: bool,
	/// Whether this entry is marked as incomplete
	pub marked_incomplete: bool,
}
