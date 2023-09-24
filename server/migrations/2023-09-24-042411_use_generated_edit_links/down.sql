ALTER TABLE event_log_history ADD COLUMN editor_link TEXT;
ALTER TABLE event_log ADD COLUMN editor_link TEXT;

ALTER TABLE events DROP COLUMN editor_link_format;