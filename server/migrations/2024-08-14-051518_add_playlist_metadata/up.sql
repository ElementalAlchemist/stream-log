-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE tags RENAME COLUMN playlist TO playlist_old;
ALTER TABLE tags ADD COLUMN playlist TEXT UNIQUE;
UPDATE tags SET playlist = playlist_old WHERE playlist_old != '';
ALTER TABLE tags DROP COLUMN playlist_old;

ALTER TABLE tags ADD COLUMN playlist_title TEXT;
ALTER TABLE tags ADD COLUMN playlist_shows_in_video_descriptions BOOLEAN;

UPDATE tags SET playlist_title = '' WHERE playlist IS NOT NULL;
UPDATE tags SET playlist_shows_in_video_descriptions = false WHERE playlist IS NOT NULL;

ALTER TABLE tags ADD CONSTRAINT playlist_has_all_or_no_data
	CHECK(
		(playlist IS NOT NULL AND playlist_title IS NOT NULL AND playlist_shows_in_video_descriptions IS NOT NULL)
		OR
		(playlist IS NULL AND playlist_title IS NULL AND playlist_shows_in_video_descriptions IS NULL)
	);