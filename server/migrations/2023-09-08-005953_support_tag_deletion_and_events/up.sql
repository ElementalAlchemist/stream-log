-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

-- Because there's not a good way to update this without destroying the tags database and because this is prior to the
-- initial full release of this program, we'll just destroy all the tags for now.

DROP TABLE event_log_tags;
DELETE FROM tags;
ALTER TABLE tags ADD COLUMN for_event TEXT NOT NULL REFERENCES events;
ALTER TABLE tags ADD COLUMN deleted BOOLEAN NOT NULL;
ALTER TABLE tags DROP CONSTRAINT unique_tag;
ALTER TABLE tags ADD CONSTRAINT unique_tag_for_event EXCLUDE (tag WITH =, for_event WITH =) WHERE (deleted = 'false');
CREATE TABLE event_log_tags (
	tag TEXT NOT NULL REFERENCES tags,
	log_entry TEXT NOT NULL REFERENCES event_log,
	PRIMARY KEY (tag, log_entry)
);