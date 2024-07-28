-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE TABLE applications (
	id TEXT PRIMARY KEY,
	name TEXT NOT NULL,
	auth_key TEXT UNIQUE, -- A null key is a revoked application.
	read_log BOOLEAN NOT NULL,
	write_links BOOLEAN NOT NULL,
	creation_user TEXT NOT NULL REFERENCES users
);