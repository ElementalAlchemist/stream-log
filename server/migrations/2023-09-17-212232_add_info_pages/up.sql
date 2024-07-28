-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE TABLE info_pages (
	id TEXT PRIMARY KEY,
	event TEXT NOT NULL REFERENCES events,
	title TEXT NOT NULL,
	contents TEXT NOT NULL,
	UNIQUE (event, title)
);