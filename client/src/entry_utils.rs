use chrono::{DateTime, NaiveDateTime, Utc};

pub const ISO_DATETIME_FORMAT_STRING: &str = "%Y-%m-%dT%H:%M:%S";

pub fn parse_time_field_value(value: &str) -> chrono::format::ParseResult<DateTime<Utc>> {
	// Inexplicably, browsers will just omit the seconds part even if seconds can be entered.
	// As such, we need to handle both formats here.
	match NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
		Ok(dt) => Ok(DateTime::from_utc(dt, Utc)),
		Err(error) => {
			if error.kind() == chrono::format::ParseErrorKind::TooShort {
				NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M").map(|dt| DateTime::from_utc(dt, Utc))
			} else {
				Err(error)
			}
		}
	}
}
