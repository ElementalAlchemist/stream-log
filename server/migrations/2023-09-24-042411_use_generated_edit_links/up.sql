-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE events ADD COLUMN editor_link_format TEXT NOT NULL DEFAULT '';
ALTER TABLE events ALTER COLUMN editor_link_format DROP DEFAULT;

ALTER TABLE event_log DROP COLUMN editor_link;
ALTER TABLE event_log_history DROP COLUMN editor_link;