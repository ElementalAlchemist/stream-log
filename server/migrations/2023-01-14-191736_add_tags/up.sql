-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE TABLE tags (
	id TEXT PRIMARY KEY,
	for_event TEXT NOT NULL REFERENCES events,
	tag TEXT NOT NULL CHECK (tag <> ''),
	description TEXT NOT NULL,
	UNIQUE (for_event, tag)
);