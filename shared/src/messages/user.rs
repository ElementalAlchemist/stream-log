use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct UserData {
	pub id: String,
	pub username: String,
	pub approval_level: UserApproval,
}

#[derive(Deserialize, Serialize)]
pub enum UserApproval {
	Unapproved,
	Approved,
	Admin,
}
