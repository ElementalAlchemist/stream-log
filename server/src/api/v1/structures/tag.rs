// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Serialize;

/// Tag object representing the tag
#[derive(Clone, Serialize)]
pub struct Tag {
	/// The tag's ID
	pub id: String,
	/// The name of the tag shown to users and used with other services
	pub tag: String,
	/// A description of what the tag is and how it's meant to be used
	pub description: String,
	/// Playlist metadata, if the tag is for a playlist
	pub playlist: Option<TagPlaylist>,
}

#[derive(Clone, Serialize)]
pub struct TagPlaylist {
	/// The ID of the playlist at the video host
	pub id: String,
	/// The title of the playlist
	pub title: String,
	/// Whether this playlist should be shown in video descriptions
	pub shows_in_video_descriptions: bool,
}
