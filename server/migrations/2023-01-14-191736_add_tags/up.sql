CREATE TABLE tags (
	id TEXT PRIMARY KEY,
	for_event TEXT NOT NULL REFERENCES events,
	tag TEXT NOT NULL CHECK (tag <> ''),
	description TEXT NOT NULL,
	UNIQUE (for_event, tag)
);