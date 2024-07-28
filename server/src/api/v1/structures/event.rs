// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Serialize;

/// Event object associated with an event.
#[derive(Serialize)]
pub struct Event {
	/// The event ID to be used for all routes that take an event ID.
	pub id: String,
	/// The event name that can be displayed to users.
	pub name: String,
}
