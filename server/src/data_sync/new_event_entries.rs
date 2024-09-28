use std::collections::HashMap;
use stream_log_shared::messages::event_log::EventLogEntry;

pub const NEW_ENTRY_COUNT: usize = 5;

#[derive(Default)]
pub struct NewEventEntries {
	pub new_entries_by_event_id: HashMap<String, Vec<EventLogEntry>>,
}
