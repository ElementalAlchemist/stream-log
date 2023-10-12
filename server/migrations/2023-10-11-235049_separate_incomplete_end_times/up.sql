ALTER TABLE event_log ADD COLUMN end_time_incomplete BOOLEAN NOT NULL DEFAULT 'f';
ALTER TABLE event_log ALTER COLUMN end_time_incomplete DROP DEFAULT;

ALTER TABLE event_log_history ADD COLUMN end_time_incomplete BOOLEAN NOT NULL DEFAULT 'f';
ALTER TABLE event_log_history ALTER COLUMN end_time_incomplete DROP DEFAULT;