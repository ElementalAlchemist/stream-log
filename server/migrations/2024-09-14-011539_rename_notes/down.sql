-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE event_log RENAME COLUMN notes TO notes_to_editor;
ALTER TABLE event_log_history RENAME COLUMN notes TO notes_to_editor;