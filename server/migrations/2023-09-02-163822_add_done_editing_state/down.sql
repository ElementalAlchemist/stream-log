-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE event_log ADD COLUMN make_video BOOLEAN NOT NULL DEFAULT 'false';
UPDATE event_log SET make_video = 'true' WHERE video_edit_state != 'no_video';
ALTER TABLE event_log ALTER COLUMN make_video DROP DEFAULT;
ALTER TABLE event_log DROP COLUMN video_edit_state;
DROP TYPE video_edit_state;