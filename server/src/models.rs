use crate::schema::users;
use diesel::{Insertable, Queryable};

#[derive(Insertable, Queryable)]
pub struct User {
	pub id: String,
	pub google_user_id: String,
	pub name: String,
}
