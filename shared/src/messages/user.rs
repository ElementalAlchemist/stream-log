use super::events::Event;
use rgb::RGB8;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct UserData {
	pub id: String,
	pub username: String,
	pub is_admin: bool,
	pub color: RGB8,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum UpdateUser {
	UpdateColor(RGB8),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserSubscriptionUpdate {
	pub user: UserData,
	pub available_events: Vec<Event>,
}
