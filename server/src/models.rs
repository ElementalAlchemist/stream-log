use crate::schema::users;
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

pub mod schema {
	pub use super::Permission;
}
