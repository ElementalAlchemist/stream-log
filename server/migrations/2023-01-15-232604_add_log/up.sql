CREATE TABLE event_log (
	id TEXT PRIMARY KEY,
	event TEXT NOT NULL REFERENCES events,
	start_time TIMESTAMP WITH TIME ZONE NOT NULL,
	end_time TIMESTAMP WITH TIME ZONE,
	event_type TEXT NOT NULL REFERENCES event_types,
	description TEXT NOT NULL,
	media_link TEXT NOT NULL,
	submitter_or_winner TEXT NOT NULL,
	notes_to_editor TEXT NOT NULL,
	editor_link TEXT NOT NULL,
	editor TEXT REFERENCES users,
	highlighted BOOLEAN NOT NULL,
	last_updated TIMESTAMP WITH TIME ZONE NOT NULL,
	last_update_user TEXT NOT NULL REFERENCES users
);

CREATE TABLE event_log_tags (
	tag TEXT REFERENCES tags ON DELETE CASCADE,
	log_entry TEXT REFERENCES event_log,
	PRIMARY KEY (tag, log_entry)
);