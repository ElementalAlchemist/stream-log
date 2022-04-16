CREATE TABLE users (
	id TEXT PRIMARY KEY,
	google_user_id TEXT UNIQUE,
	name TEXT UNIQUE
);