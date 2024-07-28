-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE event_log ADD COLUMN end_time_incomplete BOOLEAN NOT NULL DEFAULT 'f';
ALTER TABLE event_log ALTER COLUMN end_time_incomplete DROP DEFAULT;

ALTER TABLE event_log_history ADD COLUMN end_time_incomplete BOOLEAN NOT NULL DEFAULT 'f';
ALTER TABLE event_log_history ALTER COLUMN end_time_incomplete DROP DEFAULT;