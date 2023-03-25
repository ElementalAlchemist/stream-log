use crate::schema::{
	available_entry_types_for_event, entry_types, event_editors, event_log, event_log_tags, events, permission_events,
	permission_groups, tags, user_permissions, users,
};
use chrono::prelude::*;
use diesel::{Insertable, Queryable};
use diesel_derive_enum::DbEnum;
use rgb::RGB8;
use stream_log_shared::messages::permissions::PermissionLevel;

#[derive(Clone, Copy, DbEnum, Debug, Eq, PartialEq)]
pub enum Permission {
	View,
	Edit,
}

impl From<PermissionLevel> for Permission {
	fn from(level: PermissionLevel) -> Self {
		match level {
			PermissionLevel::View => Self::View,
			PermissionLevel::Edit => Self::Edit,
		}
	}
}

impl From<Permission> for PermissionLevel {
	fn from(permission: Permission) -> Self {
		match permission {
			Permission::View => Self::View,
			Permission::Edit => Self::Edit,
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

#[derive(Insertable, Queryable)]
pub struct Event {
	pub id: String,
	pub name: String,
	pub start_time: DateTime<Utc>,
}

#[derive(Insertable, Queryable)]
pub struct PermissionGroup {
	pub id: String,
	pub name: String,
}

#[derive(Insertable, Queryable)]
pub struct PermissionEvent {
	pub permission_group: String,
	pub event: String,
	pub level: Permission,
}

#[derive(Insertable, Queryable)]
pub struct UserPermission {
	pub user_id: String,
	pub permission_group: String,
}

#[derive(Insertable, Queryable)]
pub struct EntryType {
	pub id: String,
	pub name: String,
	pub color_red: i32,
	pub color_green: i32,
	pub color_blue: i32,
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

#[derive(Insertable, Queryable)]
#[diesel(table_name = available_entry_types_for_event)]
pub struct AvailableEntryType {
	pub entry_type: String,
	pub event_id: String,
}

#[derive(Insertable, Queryable)]
pub struct Tag {
	pub id: String,
	pub for_event: String,
	pub tag: String,
	pub description: String,
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
	pub make_video: bool,
	pub notes_to_editor: String,
	pub editor_link: Option<String>,
	pub editor: Option<String>,
	pub video_link: Option<String>,
	pub highlighted: bool,
	pub last_updated: DateTime<Utc>,
	pub last_update_user: String,
	pub parent: Option<String>,
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
