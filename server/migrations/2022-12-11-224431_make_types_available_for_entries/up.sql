-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

CREATE TABLE available_entry_types_for_event (
	entry_type TEXT REFERENCES entry_types,
	event_id TEXT REFERENCES events,
	PRIMARY KEY (entry_type, event_id)
);