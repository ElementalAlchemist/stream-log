use super::events::Event;
use super::user::UserData;
use crate::SYNC_VERSION;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct InitialMessage {
	pub sync_version: u32,
	pub user_data: UserDataLoad,
}

impl InitialMessage {
	pub fn new(user_data: UserDataLoad) -> Self {
		Self {
			sync_version: SYNC_VERSION,
			user_data,
		}
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub enum UserDataLoad {
	User(UserData, Vec<Event>),
	NewUser,
	MissingId,
	Error,
}
