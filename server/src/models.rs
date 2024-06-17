use crate::schema::{
	applications, available_entry_types_for_event, entry_types, event_editors, event_log, event_log_history,
	event_log_history_tags, event_log_tabs, event_log_tags, events, info_pages, permission_events, permission_groups,
	sessions, tags, user_permissions, users,
};
use chrono::prelude::*;
use diesel::{AsChangeset, Insertable, Queryable};
use diesel_derive_enum::DbEnum;
use rgb::RGB8;
use stream_log_shared::messages::admin::{
	Application as ApplicationWs, PermissionGroup as PermissionGroupWs, PermissionGroupEventAssociation,
};
use stream_log_shared::messages::entry_types::EntryType as EntryTypeWs;
use stream_log_shared::messages::event_log::{
	EndTimeData, VideoEditState as VideoEditStateWs, VideoProcessingState as VideoProcessingStateWs,
};
use stream_log_shared::messages::events::Event as EventWs;
use stream_log_shared::messages::info_pages::InfoPage as InfoPageWs;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::tags::Tag as TagWs;
use stream_log_shared::messages::user::UserData;

/// Permissions a user can have for an event, as stored in the database.
#[derive(Clone, Copy, DbEnum, Debug, Eq, PartialEq)]
#[ExistingTypePath = "crate::schema::sql_types::Permission"]
pub enum Permission {
	/// Allows viewing the event data
	View,
	/// Allows viewing and editing the event data
	Edit,
	/// Allows viewing and editing the event data and performing supervisor-specific actions
	Supervisor,
}

impl Permission {
	/// Checks whether the permission level allows editing event data
	pub fn can_edit(&self) -> bool {
		matches!(self, Self::Supervisor | Self::Edit)
	}
}

impl From<PermissionLevel> for Permission {
	fn from(level: PermissionLevel) -> Self {
		match level {
			PermissionLevel::View => Self::View,
			PermissionLevel::Edit => Self::Edit,
			PermissionLevel::Supervisor => Self::Supervisor,
		}
	}
}

impl From<Permission> for PermissionLevel {
	fn from(permission: Permission) -> Self {
		match permission {
			Permission::View => Self::View,
			Permission::Edit => Self::Edit,
			Permission::Supervisor => Self::Supervisor,
		}
	}
}

/// Edit state for a video, as stored in the database
#[derive(Clone, Copy, DbEnum, Debug, Eq, PartialEq)]
#[ExistingTypePath = "crate::schema::sql_types::VideoEditState"]
pub enum VideoEditState {
	/// State indicating that no video should be made
	NoVideo,
	/// State indicating that a video should be made, but it has not yet been completed
	MarkedForEditing,
	/// State indicating that a video was made, and the editing process has completed
	DoneEditing,
}

impl From<VideoEditStateWs> for VideoEditState {
	fn from(value: VideoEditStateWs) -> Self {
		match value {
			VideoEditStateWs::NoVideo => Self::NoVideo,
			VideoEditStateWs::MarkedForEditing => Self::MarkedForEditing,
			VideoEditStateWs::DoneEditing => Self::DoneEditing,
		}
	}
}

impl From<VideoEditState> for VideoEditStateWs {
	fn from(value: VideoEditState) -> Self {
		match value {
			VideoEditState::NoVideo => Self::NoVideo,
			VideoEditState::MarkedForEditing => Self::MarkedForEditing,
			VideoEditState::DoneEditing => Self::DoneEditing,
		}
	}
}

/// Processing state of a video, as stored in the database.
/// The processing state is updated by external systems via the API indicating a video's progress in being processed and
/// uploaded.
#[derive(Clone, Copy, DbEnum, Debug, Eq, PartialEq)]
#[ExistingTypePath = "crate::schema::sql_types::VideoProcessingState"]
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

impl From<VideoProcessingStateWs> for VideoProcessingState {
	fn from(value: VideoProcessingStateWs) -> Self {
		match value {
			VideoProcessingStateWs::Unedited => Self::Unedited,
			VideoProcessingStateWs::Edited => Self::Edited,
			VideoProcessingStateWs::Claimed => Self::Claimed,
			VideoProcessingStateWs::Finalizing => Self::Finalizing,
			VideoProcessingStateWs::Transcoding => Self::Transcoding,
			VideoProcessingStateWs::Done => Self::Done,
			VideoProcessingStateWs::Modified => Self::Modified,
			VideoProcessingStateWs::Unlisted => Self::Unlisted,
		}
	}
}

impl From<VideoProcessingState> for VideoProcessingStateWs {
	fn from(value: VideoProcessingState) -> Self {
		match value {
			VideoProcessingState::Unedited => Self::Unedited,
			VideoProcessingState::Edited => Self::Edited,
			VideoProcessingState::Claimed => Self::Claimed,
			VideoProcessingState::Finalizing => Self::Finalizing,
			VideoProcessingState::Transcoding => Self::Transcoding,
			VideoProcessingState::Done => Self::Done,
			VideoProcessingState::Modified => Self::Modified,
			VideoProcessingState::Unlisted => Self::Unlisted,
		}
	}
}

/// Database information about a user
#[derive(Insertable, Queryable)]
pub struct User {
	/// User's database ID
	pub id: String,
	/// User's ID from the OpenID identity service
	pub openid_user_id: String,
	/// User's username
	pub name: String,
	/// Whether the user is an administrator
	pub is_admin: bool,
	/// The red color value for a user's color
	pub color_red: i32,
	/// The green color value for a user's color
	pub color_green: i32,
	/// The blue color value for a user's color
	pub color_blue: i32,
}

impl User {
	/// Converts the user's individual color component values into a single color value
	pub fn color(&self) -> RGB8 {
		// Database constraints restrict the values to valid u8 values, so it's fine to unwrap these
		let red: u8 = self.color_red.try_into().unwrap();
		let green: u8 = self.color_green.try_into().unwrap();
		let blue: u8 = self.color_blue.try_into().unwrap();
		RGB8::new(red, green, blue)
	}
}

impl From<User> for UserData {
	fn from(value: User) -> Self {
		let id = value.id;
		let username = value.name;
		let is_admin = value.is_admin;

		let r: u8 = value.color_red.try_into().unwrap();
		let g: u8 = value.color_green.try_into().unwrap();
		let b: u8 = value.color_blue.try_into().unwrap();
		let color = RGB8::new(r, g, b);

		Self {
			id,
			username,
			is_admin,
			color,
		}
	}
}

/// Database information about an event
#[derive(Clone, Insertable, Queryable)]
pub struct Event {
	/// Event's ID
	pub id: String,
	/// Event's name
	pub name: String,
	/// The event's start date and time
	pub start_time: DateTime<Utc>,
	/// The URL format to use for generating editor links
	pub editor_link_format: String,
	/// The name of the first tab to show in the UI for log entries that occur before the first configured tab
	pub first_tab_name: String,
}

impl From<Event> for EventWs {
	fn from(event: Event) -> Self {
		EventWs {
			id: event.id,
			name: event.name,
			start_time: event.start_time,
			editor_link_format: event.editor_link_format,
			first_tab_name: event.first_tab_name,
		}
	}
}

/// Database information about a permission group, which links users to events for which they have permission
#[derive(Insertable, Queryable)]
pub struct PermissionGroup {
	/// Permission group's ID
	pub id: String,
	/// Name of the permission group
	pub name: String,
}

impl From<PermissionGroup> for PermissionGroupWs {
	fn from(value: PermissionGroup) -> Self {
		let id = value.id;
		let name = value.name;
		Self { id, name }
	}
}

/// Linkage between an event and a permission group
#[derive(Insertable, Queryable)]
pub struct PermissionEvent {
	/// The ID of the permission group that has permissions for the event
	pub permission_group: String,
	/// The ID of the event to have permissions granted to the group
	pub event: String,
	/// The permission to grant
	pub level: Permission,
}

impl From<PermissionEvent> for PermissionGroupEventAssociation {
	fn from(value: PermissionEvent) -> Self {
		let group = value.permission_group;
		let event = value.event;
		let permission = value.level.into();
		Self {
			group,
			event,
			permission,
		}
	}
}

/// Linkage between a user and a permission group
#[derive(Insertable, Queryable)]
pub struct UserPermission {
	/// ID of a user in the permission group
	pub user_id: String,
	/// ID of the permission group
	pub permission_group: String,
}

/// Database information on an event log entry type
#[derive(Clone, Insertable, Queryable)]
pub struct EntryType {
	/// ID of the entry type
	pub id: String,
	/// Name of the entry type
	pub name: String,
	/// Red component value of the background color
	pub color_red: i32,
	/// Green component value of the background color
	pub color_green: i32,
	/// Blue component value of the background color
	pub color_blue: i32,
	/// Description for the event type
	pub description: String,
	/// Whether log entries with this type must have an end time specified
	/// If true, the end time may be not entered yet but may not be "has no end time"
	pub require_end_time: bool,
}

impl EntryType {
	/// Converts the color components to a color value
	pub fn color(&self) -> RGB8 {
		// Database constraints restrict the values to valid u8 values, so it's fine to unwrap these
		let red: u8 = self.color_red.try_into().unwrap();
		let green: u8 = self.color_green.try_into().unwrap();
		let blue: u8 = self.color_blue.try_into().unwrap();
		RGB8::new(red, green, blue)
	}
}

impl From<EntryType> for EntryTypeWs {
	fn from(value: EntryType) -> Self {
		let color = value.color();
		let id = value.id;
		let name = value.name;
		let description = value.description;
		let require_end_time = value.require_end_time;
		Self {
			id,
			name,
			description,
			color,
			require_end_time,
		}
	}
}

/// Database linkage for an entry type being available for an event
#[derive(Insertable, Queryable)]
#[diesel(table_name = available_entry_types_for_event)]
pub struct AvailableEntryType {
	/// Entry type being made available to the event
	pub entry_type: String,
	/// Event in which the entry type is available
	pub event_id: String,
}

/// Database information on a tag
#[derive(AsChangeset, Clone, Insertable, Queryable)]
pub struct Tag {
	/// ID of the tag
	pub id: String,
	/// The tag name
	pub tag: String,
	/// A description of the tag's meaning and when it should be used
	pub description: String,
	/// The playlist ID of the playlist automatically populated from this tag.
	/// For tags that do not populate a playlist, this is set to the empty string.
	pub playlist: String,
	/// Event ID of the event in which the tag can be used
	pub for_event: String,
	/// Whether the tag has been deleted
	pub deleted: bool,
}

impl From<Tag> for TagWs {
	fn from(value: Tag) -> Self {
		let id = value.id;
		let name = value.tag;
		let description = value.description;
		let playlist = value.playlist;
		Self {
			id,
			name,
			description,
			playlist,
		}
	}
}

/// Database information on an event log entry
#[derive(Clone, Insertable, Queryable)]
#[diesel(table_name = event_log)]
pub struct EventLogEntry {
	/// ID of the entry
	pub id: String,
	/// ID of the event in which this entry was made
	pub event: String,
	/// Start time for the entry
	pub start_time: DateTime<Utc>,
	/// End time for the entry. None if the entry has no end time or the end time was not entered yet; to distinguish
	/// between those, see [end_time_incomplete].
	pub end_time: Option<DateTime<Utc>>,
	/// ID of the entry type
	pub entry_type: String,
	/// Entry's description
	pub description: String,
	/// The name of the submitter or winner related to the entry
	pub submitter_or_winner: String,
	/// Notes to the video editor for this entry
	pub notes_to_editor: String,
	/// ID of the user selected as an editor for this entry
	pub editor: Option<String>,
	/// The published video link
	pub video_link: Option<String>,
	/// ID of another entry representing this entry's parent
	pub parent: Option<String>,
	/// If this entry was deleted, ID of the user who deleted it
	pub deleted_by: Option<String>,
	/// Entry creation timestamp
	pub created_at: DateTime<Utc>,
	/// An arbitrary number used to sort entries with the same start time. Entries with numbers sort first in numerical
	/// order, followed by entries with no entered sort key.
	pub manual_sort_key: Option<i32>,
	/// The video processing state for the entry
	pub video_processing_state: Option<VideoProcessingState>,
	/// A human-readable description of errors that occurred with the video
	pub video_errors: String,
	/// Whether the entry has been marked as a poster moment
	pub poster_moment: bool,
	/// The video edit state for the entry
	pub video_edit_state: VideoEditState,
	/// Whether the entry was marked incomplete. Incomplete entries can be unmarked by a supervisor or are completed
	/// automatically with the entry of an end time and submitter/winner.
	pub marked_incomplete: bool,
	/// Any media links associated with the entry. All values in the Vec should have values.
	pub media_links: Vec<Option<String>>,
	/// Whether the end time is yet to be entered
	pub end_time_incomplete: bool,
}

impl EventLogEntry {
	/// Combines the entry end time and end time incomplete flag into a single value
	pub fn end_time_data(&self) -> EndTimeData {
		match (self.end_time, self.end_time_incomplete) {
			(Some(time), _) => EndTimeData::Time(time),
			(None, true) => EndTimeData::NotEntered,
			(None, false) => EndTimeData::NoTime,
		}
	}
}

/// A tag entered on an event log entry
#[derive(Insertable, Queryable)]
pub struct EventLogTag {
	/// ID of the tag
	pub tag: String,
	/// ID of the log entry
	pub log_entry: String,
}

/// Changeset for an event log entry
#[derive(AsChangeset, Default)]
#[diesel(table_name = event_log)]
pub struct EventLogEntryChanges {
	pub start_time: Option<DateTime<Utc>>,
	pub end_time: Option<Option<DateTime<Utc>>>,
	pub entry_type: Option<String>,
	pub description: Option<String>,
	pub submitter_or_winner: Option<String>,
	pub notes_to_editor: Option<String>,
	pub editor: Option<Option<String>>,
	pub parent: Option<Option<String>>,
	pub manual_sort_key: Option<Option<i32>>,
	pub poster_moment: Option<bool>,
	pub video_edit_state: Option<VideoEditState>,
	pub marked_incomplete: Option<bool>,
	pub media_links: Option<Vec<Option<String>>>,
	pub end_time_incomplete: Option<bool>,
}

/// A video editor for an event
#[derive(Insertable, Queryable)]
pub struct EventEditor {
	/// ID of an event for which this represents a video editor
	pub event: String,
	/// ID of the user who is a video editor for the event
	pub editor: String,
}

/// A tab in the log of an event
#[derive(Clone, Insertable, Queryable)]
pub struct EventLogTab {
	/// ID of the tab
	pub id: String,
	/// ID of the event
	pub event: String,
	/// Name of the tab
	pub name: String,
	/// Start time of the earliest entry that can appear in this tab.
	/// Log entries with a start time no earlier than this and no later than the next tab start time for the same event
	/// will appear in this tab.
	pub start_time: DateTime<Utc>,
}

/// Database information on an application that can use the API
#[derive(Insertable, Queryable)]
pub struct Application {
	/// ID of the application
	pub id: String,
	/// Name of the application
	pub name: String,
	/// Authorization key to be passed to requests from this application. None if the application was revoked.
	pub auth_key: Option<String>,
	/// Whether the application has read permissions
	pub read_log: bool,
	/// Whether the application can write links
	pub write_links: bool,
	/// ID of the user who created the application
	pub creation_user: String,
}

impl From<Application> for ApplicationWs {
	fn from(value: Application) -> Self {
		Self {
			id: value.id,
			name: value.name,
			read_log: value.read_log,
			write_links: value.write_links,
		}
	}
}

/// Database information on a historical revision of an event log entry
#[derive(Insertable, Queryable)]
#[diesel(table_name = event_log_history)]
pub struct EventLogHistoryEntry {
	/// ID of the history entry
	pub id: String,
	/// ID of the event log entry of which this is a previous revision
	pub log_entry: String,
	/// The time at which the entry was edited to these values
	pub edit_time: DateTime<Utc>,
	/// The ID of the user who edited the entry; None if this edit was made by an applicstion.
	pub edit_user: Option<String>,
	/// The ID of the application that edited the entry; None if this edit was made by a user.
	pub edit_application: Option<String>,
	pub start_time: DateTime<Utc>,
	pub end_time: Option<DateTime<Utc>>,
	pub entry_type: String,
	pub description: String,
	pub submitter_or_winner: String,
	pub notes_to_editor: String,
	pub editor: Option<String>,
	pub video_link: Option<String>,
	pub parent: Option<String>,
	pub deleted_by: Option<String>,
	pub created_at: DateTime<Utc>,
	pub manual_sort_key: Option<i32>,
	pub video_processing_state: Option<VideoProcessingState>,
	pub video_errors: String,
	pub poster_moment: bool,
	pub video_edit_state: VideoEditState,
	pub marked_incomplete: bool,
	pub media_links: Vec<Option<String>>,
	pub end_time_incomplete: bool,
}

/// The source of an edit
pub enum EditSource {
	/// Represents a user source with a user ID
	User(String),
	/// Represents an application source with an application ID
	Application(String),
}

impl EventLogHistoryEntry {
	/// Creates a log history entry from event log entry data and edit data
	pub fn new_from_event_log_entry(entry: &EventLogEntry, edit_time: DateTime<Utc>, editor: EditSource) -> Self {
		let (edit_user, edit_application) = match editor {
			EditSource::User(user_id) => (Some(user_id), None),
			EditSource::Application(app_id) => (None, Some(app_id)),
		};

		Self {
			id: cuid2::create_id(),
			log_entry: entry.id.clone(),
			edit_time,
			edit_user,
			edit_application,
			start_time: entry.start_time,
			end_time: entry.end_time,
			entry_type: entry.entry_type.clone(),
			description: entry.description.clone(),
			media_links: entry.media_links.clone(),
			submitter_or_winner: entry.submitter_or_winner.clone(),
			notes_to_editor: entry.notes_to_editor.clone(),
			editor: entry.editor.clone(),
			video_link: entry.video_link.clone(),
			parent: entry.parent.clone(),
			deleted_by: entry.deleted_by.clone(),
			created_at: entry.created_at,
			manual_sort_key: entry.manual_sort_key,
			video_processing_state: entry.video_processing_state,
			video_errors: entry.video_errors.clone(),
			poster_moment: entry.poster_moment,
			video_edit_state: entry.video_edit_state,
			marked_incomplete: entry.marked_incomplete,
			end_time_incomplete: entry.end_time_incomplete,
		}
	}
}

/// A tag associated with an event log entry history entry
#[derive(Insertable, Queryable)]
pub struct EventLogHistoryTag {
	/// ID of the tag
	pub tag: String,
	/// ID of the history entry
	pub history_log_entry: String,
}

/// An info page for event-related information
#[derive(Insertable, Queryable)]
pub struct InfoPage {
	/// ID of the page
	pub id: String,
	/// ID of the event associated with the page
	pub event: String,
	/// Page title
	pub title: String,
	/// Contents of the page, with Markdown formatting
	pub contents: String,
}

impl From<InfoPageWs> for InfoPage {
	fn from(page: InfoPageWs) -> Self {
		Self {
			id: page.id,
			event: page.event.id,
			title: page.title,
			contents: page.contents,
		}
	}
}

/// A user session
#[derive(Insertable, Queryable)]
pub struct Session {
	/// Session ID
	pub id: String,
	/// Session data
	pub data: String,
}
