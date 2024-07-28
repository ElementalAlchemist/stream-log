-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE TABLE event_log (
	id TEXT PRIMARY KEY,
	event TEXT NOT NULL REFERENCES events,
	start_time TIMESTAMP WITH TIME ZONE NOT NULL,
	end_time TIMESTAMP WITH TIME ZONE,
	entry_type TEXT NOT NULL REFERENCES entry_types,
	description TEXT NOT NULL,
	media_link TEXT NOT NULL,
	submitter_or_winner TEXT NOT NULL,
	make_video BOOLEAN NOT NULL,
	notes_to_editor TEXT NOT NULL,
	editor_link TEXT,
	editor TEXT REFERENCES users,
	video_link TEXT,
	highlighted BOOLEAN NOT NULL,
	last_updated TIMESTAMP WITH TIME ZONE NOT NULL,
	last_update_user TEXT NOT NULL REFERENCES users
);

CREATE TABLE event_log_tags (
	tag TEXT REFERENCES tags ON DELETE CASCADE,
	log_entry TEXT REFERENCES event_log,
	PRIMARY KEY (tag, log_entry)
);