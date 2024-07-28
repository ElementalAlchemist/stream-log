-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

-- Because types can't be removed for this and therefore downgrade code isn't possible, IF NOT EXISTS is added to this
-- upgrade to allow rerunning it safely on a database that was previously upgraded and downgraded.

ALTER TYPE permission ADD VALUE IF NOT EXISTS 'supervisor';