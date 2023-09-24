ALTER TABLE events ADD COLUMN editor_link_format TEXT NOT NULL DEFAULT '';
ALTER TABLE events ALTER COLUMN editor_link_format DROP DEFAULT;

ALTER TABLE event_log DROP COLUMN editor_link;
ALTER TABLE event_log_history DROP COLUMN editor_link;