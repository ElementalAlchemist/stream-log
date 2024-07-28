-- © 2022-2024 Jacob Riddle (ElementalAlchemist)
--
-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

ALTER TYPE video_state RENAME VALUE 'UNEDITED' TO 'unedited';
ALTER TYPE video_state RENAME VALUE 'EDITED' TO 'edited';
ALTER TYPE video_state RENAME VALUE 'CLAIMED' TO 'claimed';
ALTER TYPE video_state RENAME VALUE 'FINALIZING' TO 'finalizing';
ALTER TYPE video_state RENAME VALUE 'TRANSCODING' TO 'transcoding';
ALTER TYPE video_state RENAME VALUE 'DONE' TO 'done';
ALTER TYPE video_state RENAME VALUE 'MODIFIED' TO 'modified';
ALTER TYPE video_state RENAME VALUE 'UNLISTED' TO 'unlisted';