-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE DOMAIN color_byte INTEGER CONSTRAINT one_byte CHECK(VALUE >= 0 AND VALUE <= 255);

CREATE TABLE entry_types (
	id TEXT PRIMARY KEY,
	name TEXT UNIQUE NOT NULL,
	color_red color_byte NOT NULL,
	color_green color_byte NOT NULL,
	color_blue color_byte NOT NULL
);