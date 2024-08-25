// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod connection;
mod register;
mod subscription_manager;
mod subscriptions;
mod user;
mod user_profile;

pub use subscription_manager::SubscriptionManager;

use async_std::channel::SendError;
use connection::ConnectionUpdate;
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
