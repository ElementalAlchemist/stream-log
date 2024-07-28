// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

pub const ISO_DATETIME_FORMAT_STRING: &str = "%Y-%m-%dT%H:%M:%S";

pub fn parse_time_field_value(value: &str) -> chrono::format::ParseResult<DateTime<Utc>> {
	// Inexplicably, browsers will just omit the seconds part even if seconds can be entered.
	// As such, we need to handle both formats here.
	match NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
		Ok(dt) => Ok(Utc.from_utc_datetime(&dt)),
		Err(error) => {
			if error.kind() == chrono::format::ParseErrorKind::TooShort {
				NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M").map(|dt| Utc.from_utc_datetime(&dt))
			} else {
				Err(error)
			}
		}
	}
}
