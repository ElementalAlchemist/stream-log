// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Serialize};
use std::fmt;

pub mod admin;
pub mod entry_types;
pub mod event_log;
pub mod event_subscription;
pub mod events;
pub mod info_pages;
pub mod initial;
pub mod permissions;
pub mod subscriptions;
pub mod tags;
pub mod user;
pub mod user_register;

use subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionTargetUpdate, SubscriptionType,
};
use user::UpdateUser;
use user_register::{RegistrationResponse, UserRegistration};

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Clone, Deserialize, Serialize)]
pub enum FromClientMessage {
	StartSubscription(SubscriptionType),
	EndSubscription(SubscriptionType),
	SubscriptionMessage(Box<SubscriptionTargetUpdate>),
	RegistrationRequest(UserRegistration),
	UpdateProfile(UpdateUser),
}

#[derive(Deserialize, Serialize)]
pub enum FromServerMessage {
	InitialSubscriptionLoad(Box<InitialSubscriptionLoadData>),
	SubscriptionMessage(Box<SubscriptionData>),
	Unsubscribed(SubscriptionType),
	SubscriptionFailure(SubscriptionType, SubscriptionFailureInfo),
	RegistrationResponse(RegistrationResponse),
}
