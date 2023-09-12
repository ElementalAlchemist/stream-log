use super::event_log_entry::EventLogEntry;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct EventLogResponse {
	pub event_log: Vec<EventLogEntry>,
	pub retrieved_time: DateTime<Utc>,
}
