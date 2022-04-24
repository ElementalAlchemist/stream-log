use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum UserRegistration {
	CheckUsername(String),
	Finalize(UserRegistrationFinalize),
}

#[derive(Deserialize, Serialize)]
pub struct UserRegistrationFinalize {
	pub name: String,
}

#[derive(Deserialize, Serialize)]
pub struct UsernameCheckResponse {
	pub username: String,
	pub status: UsernameCheckStatus,
}

#[derive(Deserialize, Serialize)]
pub enum UsernameCheckStatus {
	Available,
	Unavailable,
}

#[derive(Deserialize, Serialize)]
pub enum RegistrationResponse {
	Success,
	UsernameInUse,
	UsernameTooLong,
}
