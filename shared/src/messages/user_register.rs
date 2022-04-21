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