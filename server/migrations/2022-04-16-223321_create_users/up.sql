-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE TABLE users (
	id TEXT PRIMARY KEY,
	openid_user_id TEXT UNIQUE NOT NULL,
	name TEXT UNIQUE NOT NULL CONSTRAINT string_not_empty CHECK (name <> ''),
	is_admin BOOLEAN NOT NULL
);