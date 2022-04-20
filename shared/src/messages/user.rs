use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum UserDataLoad {
	User(UserData),
	NewUser,
	MissingId,
	Error,
}

#[derive(Deserialize, Serialize)]
pub struct UserData {
	pub id: String,
	pub username: String,
}

#[derive(Deserialize, Serialize)]
pub enum UserRegistration {
	CheckUsername(String),
	Finalize(UserRegistrationFinalize),
}

#[derive(Deserialize, Serialize)]
pub struct UserRegistrationFinalize {
	pub name: String,
}
