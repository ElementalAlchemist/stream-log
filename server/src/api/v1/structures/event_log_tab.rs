// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::models::EventLogTab as EventLogTabDb;
use serde::Serialize;

/// The event log tab object representing a tab in which entries exist.
#[derive(Clone, Serialize)]
pub struct EventLogTab {
	/// The ID of the tab. Empty string represents the default tab for an event (for entries occurring before the start
	/// time of any configured tab).
	pub id: String,
	/// The name of the tab.
	pub name: String,
}

impl From<EventLogTabDb> for EventLogTab {
	fn from(tab: EventLogTabDb) -> Self {
		Self {
			id: tab.id,
			name: tab.name,
		}
	}
}
