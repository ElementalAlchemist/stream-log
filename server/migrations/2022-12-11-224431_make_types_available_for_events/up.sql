CREATE TABLE available_event_types_for_event (
	event_type TEXT REFERENCES event_types,
	event_id TEXT REFERENCES events,
	PRIMARY KEY (event_type, event_id)
);