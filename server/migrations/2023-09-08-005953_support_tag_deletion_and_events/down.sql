-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

DROP TABLE event_log_tags;
DELETE FROM tags WHERE deleted = 'true';
ALTER TABLE tags DROP CONSTRAINT unique_tag_for_event;
ALTER TABLE tags ADD CONSTRAINT unique_tag UNIQUE (tag);
ALTER TABLE tags DROP COLUMN for_event;
ALTER TABLE tags DROP COLUMN deleted;
CREATE TABLE event_log_tags (
	tag TEXT NOT NULL REFERENCES tags ON DELETE CASCADE,
	log_entry TEXT NOT NULL REFERENCES event_log,
	PRIMARY KEY (tag, log_entry)
);