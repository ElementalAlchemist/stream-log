// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::events::Event;
use rgb::RGB8;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct PublicUserData {
	pub id: String,
	pub username: String,
	pub color: RGB8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SelfUserData {
	pub id: String,
	pub username: String,
	pub color: RGB8,
	pub is_admin: bool,
}

impl From<SelfUserData> for PublicUserData {
	fn from(value: SelfUserData) -> Self {
		Self {
			id: value.id,
			username: value.username,
			color: value.color,
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateUser {
	pub color: RGB8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserSubscriptionUpdate {
	pub user: SelfUserData,
	pub available_events: Vec<Event>,
}
