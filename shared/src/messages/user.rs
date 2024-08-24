// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::events::Event;
use rgb::RGB8;
use serde::{Deserialize, Serialize};

/// User data sent to other users to give them information on a user and how to display their information.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct PublicUserData {
	pub id: String,
	pub username: String,
	pub color: RGB8,
}

/// User data sent to the user represented by the data, including all the settings for the user.
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

/// Update information sent when a user updates their profile settings.
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateUser {
	pub color: RGB8,
}

/// An update sent from the server any time a user's session information changes, including changes to the user data
/// itself as well as any other data relevant to the user.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserSubscriptionUpdate {
	pub user: SelfUserData,
	pub available_events: Vec<Event>,
}
