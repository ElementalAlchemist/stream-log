-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE event_log ADD COLUMN media_link TEXT NOT NULL DEFAULT '';
ALTER TABLE event_log ALTER COLUMN media_link DROP DEFAULT;

UPDATE event_log SET media_link = media_links[1] WHERE media_links[1] IS NOT NULL;

ALTER TABLE event_log DROP COLUMN media_links;

ALTER TABLE event_log_history ADD COLUMN media_link TEXT NOT NULL DEFAULT '';
ALTER TABLE event_log_history ALTER COLUMN media_link DROP DEFAULT;

UPDATE event_log_history SET media_link = media_links[1] WHERE media_links[1] IS NOT NULL;

ALTER TABLE event_log_history DROP COLUMN media_links;