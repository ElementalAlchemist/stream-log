// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
