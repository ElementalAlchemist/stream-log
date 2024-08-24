// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::user::SelfUserData;
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
	Success(SelfUserData),
	UsernameInUse,
	UsernameTooLong,
	NoUsernameSpecified,
}
