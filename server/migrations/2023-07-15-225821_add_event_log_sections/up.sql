CREATE TABLE event_log_sections (
	id TEXT PRIMARY KEY,
	event TEXT REFERENCES events NOT NULL,
	name TEXT NOT NULL,
	start_time TIMESTAMP WITH TIME ZONE NOT NULL,
	UNIQUE (event, name)
);