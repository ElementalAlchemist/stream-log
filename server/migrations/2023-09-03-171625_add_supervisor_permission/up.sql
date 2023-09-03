-- Because types can't be removed for this and therefore downgrade code isn't possible, IF NOT EXISTS is added to this
-- upgrade to allow rerunning it safely on a database that was previously upgraded and downgradded.

ALTER TYPE permission ADD VALUE IF NOT EXISTS 'supervisor';