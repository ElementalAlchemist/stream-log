-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE event_log RENAME COLUMN entry_type TO entry_type_old;
ALTER TABLE event_log ADD COLUMN entry_type TEXT REFERENCES entry_types;
UPDATE event_log SET entry_type = entry_type_old;
ALTER TABLE event_log DROP COLUMN entry_type_old;

ALTER TABLE event_log_history RENAME COLUMN entry_type TO entry_type_old;
ALTER TABLE event_log_history ADD COLUMN entry_type TEXT REFERENCES entry_types;
UPDATE event_log_history SET entry_type = entry_type_old;
ALTER TABLE event_log_history DROP COLUMN entry_type_old;