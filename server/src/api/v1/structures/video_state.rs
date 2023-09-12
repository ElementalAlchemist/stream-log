use crate::models::VideoState as VideoStateDb;
use serde::Serialize;
use std::str::FromStr;

#[derive(Serialize)]
pub enum VideoState {
	Unedited,
	Edited,
	Claimed,
	Finalizing,
	Transcoding,
	Done,
	Modified,
	Unlisted,
}

impl FromStr for VideoState {
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

impl From<VideoStateDb> for VideoState {
	fn from(db_state: VideoStateDb) -> Self {
		match db_state {
			VideoStateDb::Unedited => Self::Unedited,
			VideoStateDb::Edited => Self::Edited,
			VideoStateDb::Claimed => Self::Claimed,
			VideoStateDb::Finalizing => Self::Finalizing,
			VideoStateDb::Transcoding => Self::Transcoding,
			VideoStateDb::Done => Self::Done,
			VideoStateDb::Modified => Self::Modified,
			VideoStateDb::Unlisted => Self::Unlisted,
		}
	}
}

impl From<VideoState> for VideoStateDb {
	fn from(state: VideoState) -> Self {
		match state {
			VideoState::Unedited => Self::Unedited,
			VideoState::Edited => Self::Edited,
			VideoState::Claimed => Self::Claimed,
			VideoState::Finalizing => Self::Finalizing,
			VideoState::Transcoding => Self::Transcoding,
			VideoState::Done => Self::Done,
			VideoState::Modified => Self::Modified,
			VideoState::Unlisted => Self::Unlisted,
		}
	}
}
