CREATE DOMAIN color_byte INTEGER CONSTRAINT one_byte CHECK(VALUE >= 0 AND VALUE <= 255);

CREATE TABLE event_types (
	id TEXT PRIMARY KEY,
	name TEXT UNIQUE NOT NULL,
	color_red color_byte NOT NULL,
	color_green color_byte NOT NULL,
	color_blue color_byte NOT NULL
);