// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::events::Event;
use super::user::SelfUserData;
use crate::SYNC_VERSION;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct InitialMessage {
	pub sync_version: u32,
	pub user_data: UserDataLoad,
}

impl InitialMessage {
	pub fn new(user_data: UserDataLoad) -> Self {
		Self {
			sync_version: SYNC_VERSION,
			user_data,
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub enum UserDataLoad {
	User(SelfUserData, Vec<Event>),
	NewUser,
	MissingId,
	Error,
}
