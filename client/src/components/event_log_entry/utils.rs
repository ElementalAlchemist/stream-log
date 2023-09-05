use chrono::Duration;

/// Formats a [`Duration`] object as hours:minutes
pub fn format_duration(duration: &Duration) -> String {
	let hours = duration.num_hours();
	let minutes = (duration.num_minutes() % 60).abs();
	format!("{}:{:02}", hours, minutes)
}

/// Parses a string formatted as hhh:mm into a [`Duration`] object. If parsing fails,
/// returns a string suitable for display to the user who entered the value.
pub fn get_duration_from_formatted(formatted_duration: &str) -> Result<Duration, String> {
	let Some((hours, minutes)) = formatted_duration.split_once(':') else {
		return Err(String::from("Invalid format"));
	};

	let hours: i64 = match hours.parse() {
		Ok(hours) => hours,
		Err(error) => return Err(format!("Couldn't parse hours: {}", error)),
	};

	let minutes: i64 = match minutes.parse() {
		Ok(mins) => mins,
		Err(error) => return Err(format!("Couldn't parse minutes: {}", error)),
	};

	let duration_minutes = hours * 60 + minutes;
	Ok(Duration::minutes(duration_minutes))
}
