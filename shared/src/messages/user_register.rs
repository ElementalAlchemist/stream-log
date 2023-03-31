use super::user::UserData;
use rgb::RGB8;
use serde::{Deserialize, Serialize};

pub const USERNAME_LENGTH_LIMIT: usize = 64;

#[derive(Deserialize, Serialize)]
pub enum UserRegistration {
	CheckUsername(String),
	Finalize(UserRegistrationFinalize),
}

#[derive(Deserialize, Serialize)]
pub struct UserRegistrationFinalize {
	pub name: String,
	pub color: RGB8,
}

#[derive(Deserialize, Serialize)]
pub enum RegistrationResponse {
	UsernameCheck(UsernameCheckResponse),
	Finalize(RegistrationFinalizeResponse),
}

#[derive(Deserialize, Serialize)]
pub struct UsernameCheckResponse {
	pub username: String,
	pub available: bool,
}

#[derive(Deserialize, Serialize)]
pub enum RegistrationFinalizeResponse {
	Success(UserData),
	UsernameInUse,
	UsernameTooLong,
	NoUsernameSpecified,
}
