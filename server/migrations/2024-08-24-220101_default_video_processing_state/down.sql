-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE event_log RENAME COLUMN video_processing_state TO video_processing_state_new;
ALTER TABLE event_log_history RENAME COLUMN video_processing_state TO video_processing_state_new;

ALTER TABLE event_log ADD COLUMN video_processing_state video_processing_state;
UPDATE event_log SET video_processing_state = video_processing_state_new;
ALTER TABLE event_log DROP COLUMN video_processing_state_new;

ALTER TABLE event_log_history ADD COLUMN video_processing_state video_processing_state;
UPDATE event_log_history SET video_processing_state = video_processing_state_new;
ALTER TABLE event_log_history DROP COLUMN video_processing_state_new;