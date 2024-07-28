-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE TYPE permission AS ENUM ('view', 'edit');

CREATE TABLE events (
	id TEXT PRIMARY KEY,
	name TEXT UNIQUE NOT NULL,
	start_time TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE permission_groups (
	id TEXT PRIMARY KEY,
	name TEXT UNIQUE NOT NULL
);

CREATE TABLE permission_events (
	permission_group TEXT REFERENCES permission_groups,
	event TEXT REFERENCES events,
	level permission NOT NULL,
	PRIMARY KEY (permission_group, event)
);