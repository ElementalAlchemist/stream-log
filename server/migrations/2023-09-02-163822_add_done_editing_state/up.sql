-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE TYPE video_edit_state AS ENUM (
	'no_video',
	'marked_for_editing',
	'done_editing'
);

ALTER TABLE event_log ADD COLUMN video_edit_state video_edit_state NOT NULL DEFAULT 'no_video';
UPDATE event_log SET video_edit_state = 'marked_for_editing' WHERE make_video = 'true';
ALTER TABLE event_log ALTER COLUMN video_edit_state DROP DEFAULT;
ALTER TABLE event_log DROP COLUMN make_video;