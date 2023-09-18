CREATE TABLE info_pages (
	id TEXT PRIMARY KEY,
	event TEXT NOT NULL REFERENCES events,
	title TEXT NOT NULL,
	contents TEXT NOT NULL,
	UNIQUE (event, title)
);