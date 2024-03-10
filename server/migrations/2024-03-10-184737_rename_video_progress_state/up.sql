ALTER TYPE video_state RENAME TO video_processing_state;
ALTER TABLE event_log RENAME COLUMN video_state TO video_processing_state;
ALTER TABLE event_log_history RENAME COLUMN video_state TO video_processing_state;