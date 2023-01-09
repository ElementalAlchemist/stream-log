-- Default existing accounts to a medium gray
ALTER TABLE users ADD COLUMN color_red color_byte NOT NULL DEFAULT 127;
ALTER TABLE users ADD COLUMN color_green color_byte NOT NULL DEFAULT 127;
ALTER TABLE users ADD COLUMN color_blue color_byte NOT NULL DEFAULT 127;

-- We don't want to keep the defaults beyond the initial data population
ALTER TABLE users ALTER COLUMN color_red DROP DEFAULT;
ALTER TABLE users ALTER COLUMN color_green DROP DEFAULT;
ALTER TABLE users ALTER COLUMN color_blue DROP DEFAULT;