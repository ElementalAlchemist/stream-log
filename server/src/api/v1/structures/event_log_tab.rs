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
