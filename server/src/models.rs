use crate::schema::{default_roles, events, roles, users};
use diesel::{Insertable, Queryable};
use diesel_derive_enum::DbEnum;

#[derive(Clone, Copy, DbEnum, Debug, PartialEq)]
pub enum Approval {
	Unapproved,
	Denied,
	Approved,
	Admin,
}

impl From<Approval> for stream_log_shared::messages::user::UserApproval {
	fn from(level: Approval) -> Self {
		match level {
			Approval::Unapproved => Self::Unapproved,
			Approval::Denied => Self::Unapproved,
			Approval::Approved => Self::Approved,
			Approval::Admin => Self::Admin,
		}
	}
}

#[derive(DbEnum, Debug, PartialEq)]
pub enum Permission {
	View,
	Edit,
}

#[derive(Insertable, Queryable)]
pub struct User {
	pub id: String,
	pub google_user_id: String,
	pub name: String,
	pub account_level: Approval,
}

#[derive(Insertable, Queryable)]
pub struct Event {
	pub id: String,
	pub name: String,
}

#[derive(Insertable, Queryable)]
pub struct Role {
	pub user_id: String,
	pub event: String,
	pub permission_level: Permission,
}

#[derive(Insertable, Queryable)]
pub struct DefaultRole {
	pub event: String,
	pub permission_level: Permission,
}
