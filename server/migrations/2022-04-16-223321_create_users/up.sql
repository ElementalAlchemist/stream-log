CREATE TYPE approval AS ENUM ('unapproved', 'approved', 'admin');

CREATE TABLE users (
	id TEXT PRIMARY KEY,
	google_user_id TEXT UNIQUE NOT NULL,
	name TEXT UNIQUE NOT NULL CONSTRAINT string_not_empty CHECK (name <> ''),
	account_level approval NOT NULL
);