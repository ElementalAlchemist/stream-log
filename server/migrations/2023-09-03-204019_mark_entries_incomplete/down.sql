-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE event_log ADD COLUMN highlighted BOOLEAN NOT NULL DEFAULT 'false';
UPDATE event_log SET highlighted = 'true' WHERE marked_incomplete = 'true';
ALTER TABLE event_log ALTER COLUMN highlighted DROP DEFAULT;
ALTER TABLE event_log DROP COLUMN marked_incomplete;