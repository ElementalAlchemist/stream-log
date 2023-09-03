ALTER TABLE event_log ADD COLUMN marked_incomplete BOOLEAN NOT NULL DEFAULT 'false';
UPDATE event_log SET marked_incomplete = 'true' WHERE highlighted = 'true' AND (end_time IS NULL OR submitter_or_winner = '');
ALTER TABLE event_log ALTER COLUMN marked_incomplete DROP DEFAULT;
ALTER TABLE event_log DROP COLUMN highlighted;