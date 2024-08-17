-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE tags DROP CONSTRAINT playlist_has_all_or_no_data;

ALTER TABLE tags DROP COLUMN playlist_shows_in_video_descriptions;
ALTER TABLE tags DROP COLUMN playlist_title;

ALTER TABLE tags RENAME COLUMN playlist TO playlist_new;
ALTER TABLE tags ADD COLUMN playlist TEXT NOT NULL DEFAULT '';
UPDATE tags SET playlist = playlist_new WHERE playlist_new IS NOT NULL;
ALTER TABLE tags DROP COLUMN playlist_new;