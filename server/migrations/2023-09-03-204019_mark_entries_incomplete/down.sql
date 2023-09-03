ALTER TABLE event_log ADD COLUMN highlighted BOOLEAN NOT NULL DEFAULT 'false';
UPDATE event_log SET highlighted = 'true' WHERE marked_incomplete = 'true';
ALTER TABLE event_log ALTER COLUMN highlighted DROP DEFAULT;
ALTER TABLE event_log DROP COLUMN marked_incomplete;