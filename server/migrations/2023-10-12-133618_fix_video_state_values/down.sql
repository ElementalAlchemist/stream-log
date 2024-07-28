-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TYPE video_state RENAME VALUE 'unedited' TO 'UNEDITED';
ALTER TYPE video_state RENAME VALUE 'edited' TO 'EDITED';
ALTER TYPE video_state RENAME VALUE 'claimed' TO 'CLAIMED';
ALTER TYPE video_state RENAME VALUE 'finalizing' TO 'FINALIZING';
ALTER TYPE video_state RENAME VALUE 'transcoding' TO 'TRANSCODING';
ALTER TYPE video_state RENAME VALUE 'done' TO 'DONE';
ALTER TYPE video_state RENAME VALUE 'modified' TO 'MODIFIED';
ALTER TYPE video_state RENAME VALUE 'unlisted' TO 'UNLISTED';