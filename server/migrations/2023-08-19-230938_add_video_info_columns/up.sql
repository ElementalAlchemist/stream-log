-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE TYPE video_state AS ENUM (
	'UNEDITED',
	'EDITED',
	'CLAIMED',
	'FINALIZING',
	'TRANSCODING',
	'DONE',
	'MODIFIED',
	'UNLISTED'
);

ALTER TABLE event_log ADD COLUMN video_state video_state;
ALTER TABLE event_log ADD COLUMN video_errors TEXT NOT NULL DEFAULT '';