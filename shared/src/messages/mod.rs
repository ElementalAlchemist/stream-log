use serde::{Deserialize, Serialize};
use std::fmt;

pub mod admin;
pub mod event_types;
pub mod events;
pub mod initial;
pub mod permissions;
pub mod tags;
pub mod user;
pub mod user_register;

use admin::AdminAction;
use events::Event;

#[derive(Deserialize, Serialize)]
pub enum DataError {
	DatabaseError,
	ServerError,
}

impl fmt::Display for DataError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::DatabaseError => write!(f, "A database interaction failed"),
			Self::ServerError => write!(f, "The server failed to process"),
		}
	}
}

pub type DataMessage<T> = Result<T, DataError>;

#[derive(Deserialize, Serialize)]
pub enum RequestMessage {
	ListAvailableEvents,
	SwitchToEvent(Event),
	Admin(AdminAction),
}
