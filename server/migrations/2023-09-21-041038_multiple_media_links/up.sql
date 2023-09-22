ALTER TABLE event_log ADD COLUMN media_links TEXT[] NOT NULL DEFAULT '{}' CHECK (array_position(media_links, NULL) IS NULL);
ALTER TABLE event_log ALTER COLUMN media_links DROP DEFAULT;

UPDATE event_log SET media_links[1] = media_link WHERE media_link != '';

ALTER TABLE event_log DROP COLUMN media_link;

ALTER TABLE event_log_history ADD COLUMN media_links TEXT[] NOT NULL DEFAULT '{}' CHECK (array_position(media_links, NULL) IS NULL);
ALTER TABLE event_log_history ALTER COLUMN media_links DROP DEFAULT;

UPDATE event_log_history SET media_links[1] = media_link WHERE media_link != '';

ALTER TABLE event_log_history DROP COLUMN media_link;