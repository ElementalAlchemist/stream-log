use serde::{Deserialize, Serialize};

/// Entry type object used to define entry types.
#[derive(Clone, Deserialize, Serialize)]
pub struct EntryType {
	/// The entry type's ID
	pub id: String,
	/// The name of the entry type
	pub name: String,
	/// The red compoment of the background color for this entry type.
	pub color_red: u8,
	/// The green component of the background color for this entry type.
	pub color_green: u8,
	/// The blue component of the bakground color for this entry type.
	pub color_blue: u8,
}
