ALTER TABLE event_log ADD COLUMN make_video BOOLEAN NOT NULL DEFAULT 'false';
UPDATE event_log SET make_video = 'true' WHERE video_edit_state != 'no_video';
ALTER TABLE event_log ALTER COLUMN make_video DROP DEFAULT;
ALTER TABLE event_log DROP COLUMN video_edit_state;
DROP TYPE video_edit_state;