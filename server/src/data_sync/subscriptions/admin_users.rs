// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::data_sync::connection::ConnectionUpdate;
use crate::data_sync::UserDataUpdate;
use crate::data_sync::{HandleConnectionError, SubscriptionManager};
use crate::models::User;
use crate::schema::users;
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_users(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message =
			FromServerMessage::SubscriptionFailure(SubscriptionType::AdminUsers, SubscriptionFailureInfo::NotAllowed);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = db_connection.lock().await;
	let all_users: QueryResult<Vec<User>> = users::table.load(&mut *db_connection);
	let all_users = match all_users {
		Ok(users) => users,
		Err(error) => {
			tide::log::error!("A database error occurred getting the user list: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminUsers,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_user_subscription(connection_id, conn_update_tx.clone())
		.await;

	let all_user_data: Vec<UserData> = all_users.into_iter().map(|user| user.into()).collect();
	let message =
		FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminUsers(all_user_data)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_admin_users_message(
	db_connection: Arc<Mutex<PgConnection>>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	modified_user: &UserData,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.is_subscribed_to_admin_users(connection_id)
		.await
	{
		return;
	}

	let color_red: i32 = modified_user.color.r.into();
	let color_green: i32 = modified_user.color.g.into();
	let color_blue: i32 = modified_user.color.b.into();
	{
		let mut db_connection = db_connection.lock().await;
		let db_result = diesel::update(users::table)
			.filter(users::id.eq(&modified_user.id))
			.set((
				users::name.eq(&modified_user.username),
				users::is_admin.eq(modified_user.is_admin),
				users::color_red.eq(color_red),
				users::color_green.eq(color_green),
				users::color_blue.eq(color_blue),
			))
			.execute(&mut *db_connection);
		if let Err(error) = db_result {
			tide::log::error!("A database error occurred updating a user: {}", error);
			return;
		}
	}

	let mut subscription_manager = subscription_manager.lock().await;
	let admin_message = SubscriptionData::AdminUsersUpdate(modified_user.clone());
	let send_result = subscription_manager.broadcast_admin_user_message(admin_message).await;
	if let Err(error) = send_result {
		tide::log::error!("Failed to send admin message for user update: {}", error);
	}
	let user_message = UserDataUpdate::User(modified_user.clone());
	subscription_manager
		.send_message_to_user(&modified_user.id, user_message)
		.await;
}
