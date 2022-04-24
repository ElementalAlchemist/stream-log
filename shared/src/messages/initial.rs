use crate::SYNC_VERSION;
use super::user::UserData;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
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

#[derive(Deserialize, Serialize)]
pub enum UserDataLoad {
	User(UserData),
	NewUser,
	MissingId,
	Error,
}
