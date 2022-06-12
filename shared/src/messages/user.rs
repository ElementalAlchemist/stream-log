use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct UserData {
	pub id: String,
	pub username: String,
	pub approval_level: UserApproval,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub enum UserApproval {
	Unapproved,
	Approved,
	Admin,
}
