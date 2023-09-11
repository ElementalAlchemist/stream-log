use serde::{Deserialize, Serialize};

/// User object representing a user.
#[derive(Clone, Deserialize, Serialize)]
pub struct User {
	/// The user's ID
	pub id: String,
	/// The username for the user
	pub username: String,
	/// The red component of the user's color
	pub color_red: u8,
	/// The green component of the user's color
	pub color_green: u8,
	/// The blue component of the user's color
	pub color_blue: u8,
}
