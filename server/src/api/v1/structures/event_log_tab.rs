// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::models::EventLogTab as EventLogTabDb;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct EventLogTab {
	id: String,
	name: String,
}

impl From<EventLogTabDb> for EventLogTab {
	fn from(tab: EventLogTabDb) -> Self {
		Self {
			id: tab.id,
			name: tab.name,
		}
	}
}
