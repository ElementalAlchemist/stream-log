use crate::schema::{default_roles, events, roles, users};
use diesel::{Insertable, Queryable};
use diesel_derive_enum::DbEnum;

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
}

#[derive(Insertable, Queryable)]
pub struct Event {
	pub id: String,
	pub name: String
}

#[derive(Insertable, Queryable)]
pub struct Role {
	pub user_id: String,
	pub event: String,
	pub permission_level: Permission
}

#[derive(Insertable, Queryable)]
pub struct DefaultRole {
	pub event: String,
	pub permission_level: Permission
}