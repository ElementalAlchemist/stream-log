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
	/// Playlist ID, if the tag is for a playlist
	pub playlist: Option<String>,
}
