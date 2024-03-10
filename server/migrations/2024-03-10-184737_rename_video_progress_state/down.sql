ALTER TABLE event_log_history RENAME COLUMN video_processing_state TO video_state;
ALTER TABLE event_log RENAME COLUMN video_processing_state TO video_state;
ALTER TYPE video_processing_state RENAME TO video_state;