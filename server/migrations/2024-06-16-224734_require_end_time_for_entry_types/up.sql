ALTER TABLE entry_types ADD COLUMN require_end_time BOOLEAN NOT NULL DEFAULT 'false';
ALTER TABLE entry_types ALTER COLUMN require_end_time DROP DEFAULT;