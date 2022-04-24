CREATE TYPE permission AS ENUM ('view', 'edit');

CREATE TABLE events (
	id TEXT PRIMARY KEY,
	name TEXT UNIQUE NOT NULL
);

CREATE TABLE roles (
	user_id TEXT REFERENCES users,
	event TEXT REFERENCES events,
	permission_level permission NOT NULL,
	PRIMARY KEY (user_id, event)
);

CREATE TABLE default_roles (
	event TEXT REFERENCES events PRIMARY KEY,
	permission_level permission NOT NULL
);