-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

DROP TABLE event_log_history_tags;
DROP TABLE event_log_history;

ALTER TABLE event_log ADD COLUMN last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW();
ALTER TABLE event_log ADD COLUMN last_update_user TIMESTAMP WITH TIME ZONE;
ALTER TABLE event_log ALTER COLUMN last_updated DROP DEFAULT;