// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::event_log_entry::EventLogEntry;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct EventLogResponse {
	pub event_log: Vec<EventLogEntry>,
	pub retrieved_time: DateTime<Utc>,
}
