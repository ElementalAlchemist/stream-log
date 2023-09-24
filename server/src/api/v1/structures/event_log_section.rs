use crate::models::EventLogSection as EventLogSectionDb;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct EventLogSection {
	id: String,
	name: String,
}

impl From<EventLogSectionDb> for EventLogSection {
	fn from(section: EventLogSectionDb) -> Self {
		Self {
			id: section.id,
			name: section.name,
		}
	}
}
