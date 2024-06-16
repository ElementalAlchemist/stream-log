pub mod connection;
mod register;
mod subscription_manager;
mod subscriptions;
mod user;
mod user_profile;

pub use subscription_manager::SubscriptionManager;

use crate::data_sync::connection::ConnectionUpdate;
use async_std::channel::SendError;
use user::UserDataUpdate;

pub enum HandleConnectionError {
	ConnectionClosed,
	SendError(tide::Error),
	ChannelError,
}

impl From<tide::Error> for HandleConnectionError {
	fn from(error: tide::Error) -> Self {
		Self::SendError(error)
	}
}

impl From<SendError<ConnectionUpdate>> for HandleConnectionError {
	fn from(_: SendError<ConnectionUpdate>) -> Self {
		Self::ChannelError
	}
}
