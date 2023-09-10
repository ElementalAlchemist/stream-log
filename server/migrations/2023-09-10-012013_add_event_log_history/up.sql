ALTER TABLE event_log ALTER COLUMN video_errors DROP DEFAULT;

CREATE TABLE event_log_history (
	id TEXT PRIMARY KEY,
	log_entry TEXT NOT NULL REFERENCES event_log,
	edit_time TIMESTAMP WITH TIME ZONE NOT NULL,
	edit_user TEXT REFERENCES users,
	edit_application TEXT REFERENCES applications,
	start_time TIMESTAMP WITH TIME ZONE NOT NULL,
	end_time TIMESTAMP WITH TIME ZONE,
	entry_type TEXT NOT NULL REFERENCES entry_types,
	description TEXT NOT NULL,
	media_link TEXT NOT NULL,
	submitter_or_winner TEXT NOT NULL,
	notes_to_editor TEXT NOT NULL,
	editor_link TEXT,
	editor TEXT REFERENCES users,
	video_link TEXT,
	parent TEXT REFERENCES event_log,
	deleted_by TEXT REFERENCES users,
	created_at TIMESTAMP WITH TIME ZONE NOT NULL,
	manual_sort_key INTEGER,
	video_state video_state,
	video_errors TEXT NOT NULL,
	poster_moment BOOLEAN NOT NULL,
	video_edit_state video_edit_state NOT NULL,
	marked_incomplete BOOLEAN NOT NULL,
	CHECK (edit_user IS NOT NULL OR edit_application IS NOT NULL)
);

CREATE TABLE event_log_history_tags (
	tag TEXT NOT NULL REFERENCES tags,
	history_log_entry TEXT NOT NULL REFERENCES event_log_history,
	PRIMARY KEY (tag, history_log_entry)
);

ALTER TABLE event_log DROP COLUMN last_updated;
ALTER TABLE event_log DROP COLUMN last_update_user;