DROP TABLE event_log_history_tags;
DROP TABLE event_log_history;

ALTER TABLE event_log ADD COLUMN last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW();
ALTER TABLE event_log ADD COLUMN last_update_user TIMESTAMP WITH TIME ZONE;
ALTER TABLE event_log ALTER COLUMN last_updated DROP DEFAULT;