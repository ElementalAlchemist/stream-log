// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod admin_applications;
pub mod admin_editors;
pub mod admin_entry_types;
pub mod admin_events;
pub mod admin_pages;
pub mod admin_permission_groups;
pub mod admin_tabs;
pub mod admin_users;
pub mod events;

use crate::data_sync::{ConnectionUpdate, HandleConnectionError};
use crate::database::log_lost_db_connection;
use async_std::channel::Sender;
use r2d2::Error as R2D2Error;
use stream_log_shared::messages::subscriptions::{SubscriptionFailureInfo, SubscriptionType};
use stream_log_shared::messages::{DataError, FromServerMessage};

/// Logs the database connection lost message to the web server log and sends the appropriate response to the client.
async fn send_lost_db_connection_subscription_response(
	error: R2D2Error,
	conn_update_tx: &Sender<ConnectionUpdate>,
	subscription_type: SubscriptionType,
) -> Result<(), HandleConnectionError> {
	log_lost_db_connection(error);
	let message = FromServerMessage::SubscriptionFailure(
		subscription_type,
		SubscriptionFailureInfo::Error(DataError::DatabaseError),
	);
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;
	Ok(())
}
