// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::models::VideoProcessingState as VideoProcessingStateDb;
use serde::Serialize;
use std::str::FromStr;

#[derive(Serialize)]
pub enum VideoProcessingState {
	Unedited,
	Edited,
	Claimed,
	Finalizing,
	Transcoding,
	Done,
	Modified,
	Unlisted,
}

impl FromStr for VideoProcessingState {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_uppercase().as_str() {
			"UNEDITED" => Ok(Self::Unedited),
			"EDITED" => Ok(Self::Edited),
			"CLAIMED" => Ok(Self::Claimed),
			"FINALIZING" => Ok(Self::Finalizing),
			"TRANSCODING" => Ok(Self::Transcoding),
			"DONE" => Ok(Self::Done),
			"MODIFIED" => Ok(Self::Modified),
			"UNLISTED" => Ok(Self::Unlisted),
			_ => Err(()),
		}
	}
}

impl From<VideoProcessingStateDb> for VideoProcessingState {
	fn from(db_state: VideoProcessingStateDb) -> Self {
		match db_state {
			VideoProcessingStateDb::Unedited => Self::Unedited,
			VideoProcessingStateDb::Edited => Self::Edited,
			VideoProcessingStateDb::Claimed => Self::Claimed,
			VideoProcessingStateDb::Finalizing => Self::Finalizing,
			VideoProcessingStateDb::Transcoding => Self::Transcoding,
			VideoProcessingStateDb::Done => Self::Done,
			VideoProcessingStateDb::Modified => Self::Modified,
			VideoProcessingStateDb::Unlisted => Self::Unlisted,
		}
	}
}

impl From<VideoProcessingState> for VideoProcessingStateDb {
	fn from(state: VideoProcessingState) -> Self {
		match state {
			VideoProcessingState::Unedited => Self::Unedited,
			VideoProcessingState::Edited => Self::Edited,
			VideoProcessingState::Claimed => Self::Claimed,
			VideoProcessingState::Finalizing => Self::Finalizing,
			VideoProcessingState::Transcoding => Self::Transcoding,
			VideoProcessingState::Done => Self::Done,
			VideoProcessingState::Modified => Self::Modified,
			VideoProcessingState::Unlisted => Self::Unlisted,
		}
	}
}
