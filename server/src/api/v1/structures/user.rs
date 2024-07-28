// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Serialize;

/// User object representing a user.
#[derive(Clone, Serialize)]
pub struct User {
	/// The user's ID
	pub id: String,
	/// The username for the user
	pub username: String,
	/// The red component of the user's color
	pub color_red: u8,
	/// The green component of the user's color
	pub color_green: u8,
	/// The blue component of the user's color
	pub color_blue: u8,
}
