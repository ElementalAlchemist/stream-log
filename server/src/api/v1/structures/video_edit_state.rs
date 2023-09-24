use crate::models::VideoEditState as VideoEditStateDb;
use serde::Serialize;

#[derive(Serialize)]
pub enum VideoEditState {
	NoVideo,
	MarkedForEditing,
	DoneEditing,
}

impl From<VideoEditStateDb> for VideoEditState {
	fn from(state: VideoEditStateDb) -> Self {
		match state {
			VideoEditStateDb::NoVideo => Self::NoVideo,
			VideoEditStateDb::MarkedForEditing => Self::MarkedForEditing,
			VideoEditStateDb::DoneEditing => Self::DoneEditing,
		}
	}
}
