use crate::schema::{
	applications, available_entry_types_for_event, entry_types, event_editors, event_log, event_log_history,
	event_log_history_tags, event_log_sections, event_log_tags, events, permission_events, permission_groups, tags,
	user_permissions, users,
};
use chrono::prelude::*;
use diesel::{AsChangeset, Insertable, Queryable};
use diesel_derive_enum::DbEnum;
use rgb::RGB8;
use stream_log_shared::messages::admin::{
	Application as ApplicationWs, PermissionGroup as PermissionGroupWs, PermissionGroupEventAssociation,
};
use stream_log_shared::messages::entry_types::EntryType as EntryTypeWs;
use stream_log_shared::messages::event_log::{VideoEditState as VideoEditStateWs, VideoState as VideoStateWs};
use stream_log_shared::messages::events::Event as EventWs;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::tags::Tag as TagWs;
use stream_log_shared::messages::user::UserData;

#[derive(Clone, Copy, DbEnum, Debug, Eq, PartialEq)]
#[ExistingTypePath = "crate::schema::sql_types::Permission"]
pub enum Permission {
	View,
	Edit,
	Supervisor,
}

impl Permission {
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

#[derive(Clone, Copy, DbEnum, Debug, Eq, PartialEq)]
#[ExistingTypePath = "crate::schema::sql_types::VideoEditState"]
pub enum VideoEditState {
	NoVideo,
	MarkedForEditing,
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

#[derive(Clone, Copy, DbEnum, Debug, Eq, PartialEq)]
#[ExistingTypePath = "crate::schema::sql_types::VideoState"]
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

impl From<VideoStateWs> for VideoState {
	fn from(value: VideoStateWs) -> Self {
		match value {
			VideoStateWs::Unedited => Self::Unedited,
			VideoStateWs::Edited => Self::Edited,
			VideoStateWs::Claimed => Self::Claimed,
			VideoStateWs::Finalizing => Self::Finalizing,
			VideoStateWs::Transcoding => Self::Transcoding,
			VideoStateWs::Done => Self::Done,
			VideoStateWs::Modified => Self::Modified,
			VideoStateWs::Unlisted => Self::Unlisted,
		}
	}
}

impl From<VideoState> for VideoStateWs {
	fn from(value: VideoState) -> Self {
		match value {
			VideoState::Unedited => Self::Unedited,
			VideoState::Edited => Self::Edited,
			VideoState::Claimed => Self::Claimed,
			VideoState::Finalizing => Self::Finalizing,
			VideoState::Transcoding => Self::Transcoding,
			VideoState::Done => Self::Done,
			VideoState::Modified => Self::Modified,
			VideoState::Unlisted => Self::Unlisted,
		}
	}
}

#[derive(Insertable, Queryable)]
pub struct User {
	pub id: String,
	pub openid_user_id: String,
	pub name: String,
	pub is_admin: bool,
	pub color_red: i32,
	pub color_green: i32,
	pub color_blue: i32,
}

impl User {
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

#[derive(Clone, Insertable, Queryable)]
pub struct Event {
	pub id: String,
	pub name: String,
	pub start_time: DateTime<Utc>,
}

impl From<Event> for EventWs {
	fn from(value: Event) -> Self {
		EventWs {
			id: value.id,
			name: value.name,
			start_time: value.start_time,
		}
	}
}

#[derive(Insertable, Queryable)]
pub struct PermissionGroup {
	pub id: String,
	pub name: String,
}

impl From<PermissionGroup> for PermissionGroupWs {
	fn from(value: PermissionGroup) -> Self {
		let id = value.id;
		let name = value.name;
		Self { id, name }
	}
}

#[derive(Insertable, Queryable)]
pub struct PermissionEvent {
	pub permission_group: String,
	pub event: String,
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

#[derive(Insertable, Queryable)]
pub struct UserPermission {
	pub user_id: String,
	pub permission_group: String,
}

#[derive(Clone, Insertable, Queryable)]
pub struct EntryType {
	pub id: String,
	pub name: String,
	pub color_red: i32,
	pub color_green: i32,
	pub color_blue: i32,
	pub description: String,
}

impl EntryType {
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
		Self {
			id,
			name,
			description,
			color,
		}
	}
}

#[derive(Insertable, Queryable)]
#[diesel(table_name = available_entry_types_for_event)]
pub struct AvailableEntryType {
	pub entry_type: String,
	pub event_id: String,
}

#[derive(AsChangeset, Clone, Insertable, Queryable)]
pub struct Tag {
	pub id: String,
	pub tag: String,
	pub description: String,
	pub playlist: String,
	pub for_event: String,
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

#[derive(Insertable, Queryable)]
#[diesel(table_name = event_log)]
pub struct EventLogEntry {
	pub id: String,
	pub event: String,
	pub start_time: DateTime<Utc>,
	pub end_time: Option<DateTime<Utc>>,
	pub entry_type: String,
	pub description: String,
	pub media_link: String,
	pub submitter_or_winner: String,
	pub notes_to_editor: String,
	pub editor_link: Option<String>,
	pub editor: Option<String>,
	pub video_link: Option<String>,
	pub parent: Option<String>,
	pub deleted_by: Option<String>,
	pub created_at: DateTime<Utc>,
	pub manual_sort_key: Option<i32>,
	pub video_state: Option<VideoState>,
	pub video_errors: String,
	pub poster_moment: bool,
	pub video_edit_state: VideoEditState,
	pub marked_incomplete: bool,
}

#[derive(Insertable, Queryable)]
pub struct EventLogTag {
	pub tag: String,
	pub log_entry: String,
}

#[derive(Insertable, Queryable)]
pub struct EventEditor {
	pub event: String,
	pub editor: String,
}

#[derive(Insertable, Queryable)]
pub struct EventLogSection {
	pub id: String,
	pub event: String,
	pub name: String,
	pub start_time: DateTime<Utc>,
}

#[derive(Insertable, Queryable)]
pub struct Application {
	pub id: String,
	pub name: String,
	pub auth_key: Option<String>,
	pub read_log: bool,
	pub write_links: bool,
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

#[derive(Insertable, Queryable)]
#[diesel(table_name = event_log_history)]
pub struct EventLogHistoryEntry {
	pub id: String,
	pub log_entry: String,
	pub edit_time: DateTime<Utc>,
	pub edit_user: Option<String>,
	pub edit_application: Option<String>,
	pub start_time: DateTime<Utc>,
	pub end_time: Option<DateTime<Utc>>,
	pub entry_type: String,
	pub description: String,
	pub media_link: String,
	pub submitter_or_winner: String,
	pub notes_to_editor: String,
	pub editor_link: Option<String>,
	pub editor: Option<String>,
	pub video_link: Option<String>,
	pub parent: Option<String>,
	pub deleted_by: Option<String>,
	pub created_at: DateTime<Utc>,
	pub manual_sort_key: Option<i32>,
	pub video_state: Option<VideoState>,
	pub video_errors: String,
	pub poster_moment: bool,
	pub video_edit_state: VideoEditState,
	pub marked_incomplete: bool,
}

pub enum EditSource {
	User(String),
	Application(String),
}

impl EventLogHistoryEntry {
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
			media_link: entry.media_link.clone(),
			submitter_or_winner: entry.submitter_or_winner.clone(),
			notes_to_editor: entry.notes_to_editor.clone(),
			editor_link: entry.editor_link.clone(),
			editor: entry.editor.clone(),
			video_link: entry.video_link.clone(),
			parent: entry.parent.clone(),
			deleted_by: entry.deleted_by.clone(),
			created_at: entry.created_at,
			manual_sort_key: entry.manual_sort_key,
			video_state: entry.video_state,
			video_errors: entry.video_errors.clone(),
			poster_moment: entry.poster_moment,
			video_edit_state: entry.video_edit_state,
			marked_incomplete: entry.marked_incomplete,
		}
	}
}

#[derive(Insertable, Queryable)]
pub struct EventLogHistoryTag {
	pub tag: String,
	pub history_log_entry: String,
}
