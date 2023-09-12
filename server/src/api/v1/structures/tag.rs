use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Tag {
	pub id: String,
	pub tag: String,
	pub description: String,
	pub playlist: Option<String>,
}
