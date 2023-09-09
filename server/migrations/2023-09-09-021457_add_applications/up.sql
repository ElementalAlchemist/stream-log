CREATE TABLE applications (
	id TEXT PRIMARY KEY,
	name TEXT NOT NULL,
	auth_key TEXT UNIQUE, -- A null key is a revoked application.
	read_log BOOLEAN NOT NULL,
	write_links BOOLEAN NOT NULL,
	creation_user TEXT NOT NULL REFERENCES users
);