DROP TABLE event_log_tags;
DELETE FROM tags WHERE deleted = 'true';
ALTER TABLE tags DROP CONSTRAINT unique_tag_for_event;
ALTER TABLE tags ADD CONSTRAINT unique_tag UNIQUE (tag);
ALTER TABLE tags DROP COLUMN for_event;
ALTER TABLE tags DROP COLUMN deleted;
CREATE TABLE event_log_tags (
	tag TEXT NOT NULL REFERENCES tags ON DELETE CASCADE,
	log_entry TEXT NOT NULL REFERENCES event_log,
	PRIMARY KEY (tag, log_entry)
);