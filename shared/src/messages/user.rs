// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::events::Event;
use rgb::RGB8;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct UserData {
	pub id: String,
	pub username: String,
	pub is_admin: bool,
	pub color: RGB8,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum UpdateUser {
	UpdateColor(RGB8),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserSubscriptionUpdate {
	pub user: UserData,
	pub available_events: Vec<Event>,
}
