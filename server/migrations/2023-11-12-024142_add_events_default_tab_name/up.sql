ALTER TABLE events ADD COLUMN default_first_tab_name TEXT NOT NULL DEFAULT '';
ALTER TABLE events ALTER COLUMN default_first_tab_name DROP DEFAULT;