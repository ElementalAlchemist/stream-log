use crate::schema::{event_types, events, permission_events, permission_groups, user_permissions, users};
use chrono::prelude::*;
use diesel::{Insertable, Queryable};
use diesel_derive_enum::DbEnum;
use rgb::RGB8;
use stream_log_shared::messages::permissions::PermissionLevel;

#[derive(DbEnum, Debug, Eq, PartialEq)]
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
pub struct EventType {
	pub id: String,
	pub name: String,
	pub color_red: i32,
	pub color_green: i32,
	pub color_blue: i32,
}

impl EventType {
	pub fn color(&self) -> RGB8 {
		// Database constraints restrict the values to valid u8 values, so it's fine to unwrap these
		let red: u8 = self.color_red.try_into().unwrap();
		let green: u8 = self.color_green.try_into().unwrap();
		let blue: u8 = self.color_blue.try_into().unwrap();
		RGB8::new(red, green, blue)
	}
}
