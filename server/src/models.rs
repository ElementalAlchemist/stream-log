use diesel::Queryable;

#[derive(Queryable)]
pub struct User {
	pub id: String,
	pub google_user_id: String,
	pub name: String,
}
