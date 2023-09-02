CREATE TYPE video_edit_state AS ENUM (
	'no_video',
	'marked_for_editing',
	'done_editing'
);

ALTER TABLE event_log ADD COLUMN video_edit_state video_edit_state NOT NULL DEFAULT 'no_video';
UPDATE event_log SET video_edit_state = 'marked_for_editing' WHERE make_video = 'true';
ALTER TABLE event_log ALTER COLUMN video_edit_state DROP DEFAULT;
ALTER TABLE event_log DROP COLUMN make_video;