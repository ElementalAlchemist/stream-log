CREATE TABLE available_entry_types_for_event (
	entry_type TEXT REFERENCES entry_types,
	event_id TEXT REFERENCES events,
	PRIMARY KEY (entry_type, event_id)
);