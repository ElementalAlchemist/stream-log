// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use chrono::Duration;

/// Formats a [`Duration`] object as hours:minutes
pub fn format_duration(duration: &Duration) -> String {
	let mut hours = duration.num_hours();
	let mut minutes = duration.num_minutes() % 60;
	let sign = if hours < 0 || minutes < 0 {
		hours = hours.abs();
		minutes = minutes.abs();
		"-"
	} else {
		""
	};
	format!("{}{}:{:02}", sign, hours, minutes)
}

/// Parses a string formatted as hhh:mm into a [`Duration`] object. If parsing fails,
/// returns a string suitable for display to the user who entered the value.
pub fn get_duration_from_formatted(formatted_duration: &str) -> Result<Duration, String> {
	let Some((hours, minutes)) = formatted_duration.split_once(':') else {
		return Err(String::from("Invalid format"));
	};

	let is_negative = match hours.chars().next() {
		Some(c) => c == '-',
		None => false,
	};
	let hours: i64 = match hours.parse() {
		Ok(hours) => hours,
		Err(error) => return Err(format!("Couldn't parse hours: {}", error)),
	};

	let mut minutes: i64 = match minutes.parse() {
		Ok(mins) => mins,
		Err(error) => return Err(format!("Couldn't parse minutes: {}", error)),
	};

	if is_negative {
		if hours > 0 {
			return Err(format!(
				"Hour parsing went wrong: detected negative duration but parsed hours as {}",
				hours
			));
		}

		minutes = -minutes;
	}

	let duration_minutes = hours * 60 + minutes;
	Ok(Duration::minutes(duration_minutes))
}
