CREATE TYPE video_state AS ENUM (
	'UNEDITED',
	'EDITED',
	'CLAIMED',
	'FINALIZING',
	'TRANSCODING',
	'DONE',
	'MODIFIED',
	'UNLISTED'
);

ALTER TABLE event_log ADD COLUMN video_state video_state;
ALTER TABLE event_log ADD COLUMN video_errors TEXT NOT NULL DEFAULT '';