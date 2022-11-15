CREATE TYPE permission AS ENUM ('view', 'edit');

CREATE TABLE events (
	id TEXT PRIMARY KEY,
	name TEXT UNIQUE NOT NULL,
	start_time TIMESTAMP NOT NULL
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