-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TABLE entry_types ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE entry_types ALTER COLUMN description DROP DEFAULT;