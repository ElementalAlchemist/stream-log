-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

-- There's not a good migration that makes a nullable foreign key column non-nullable again, so for the downward
-- migration, we'll just ignore the nullability of the column.
SELECT 1;