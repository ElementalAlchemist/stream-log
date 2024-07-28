// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Serialize;

/// Entry type object used to define entry types.
#[derive(Clone, Serialize)]
pub struct EntryType {
	/// The entry type's ID
	pub id: String,
	/// The name of the entry type
	pub name: String,
	/// The red compoment of the background color for this entry type.
	pub color_red: u8,
	/// The green component of the background color for this entry type.
	pub color_green: u8,
	/// The blue component of the bakground color for this entry type.
	pub color_blue: u8,
}
