-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE event_log_history RENAME COLUMN video_processing_state TO video_state;
ALTER TABLE event_log RENAME COLUMN video_processing_state TO video_state;
ALTER TYPE video_processing_state RENAME TO video_state;