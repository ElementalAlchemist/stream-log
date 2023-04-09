use super::user::UserData;
use rgb::RGB8;
use serde::{Deserialize, Serialize};

pub const USERNAME_LENGTH_LIMIT: usize = 64;

/// Request data from the client while registering
#[derive(Deserialize, Serialize)]
pub enum UserRegistration {
	CheckUsername(String),
	Finalize(UserRegistrationFinalize),
}

/// Data from the client when trying to register an account
#[derive(Deserialize, Serialize)]
pub struct UserRegistrationFinalize {
	pub name: String,
	pub color: RGB8,
}

/// Response data from the server related to registration
#[derive(Deserialize, Serialize)]
pub enum RegistrationResponse {
	UsernameCheck(UsernameCheckResponse),
	Finalize(RegistrationFinalizeResponse),
}

/// Response data from the server for a username check
#[derive(Deserialize, Serialize)]
pub struct UsernameCheckResponse {
	pub username: String,
	pub available: bool,
}

/// Response data from the server for a full registration attempt
#[derive(Deserialize, Serialize)]
pub enum RegistrationFinalizeResponse {
	Success(UserData),
	UsernameInUse,
	UsernameTooLong,
	NoUsernameSpecified,
}
