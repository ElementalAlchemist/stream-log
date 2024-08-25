// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::user::UserDataUpdate;
use super::{HandleConnectionError, SubscriptionManager};
use crate::schema::users;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use stream_log_shared::messages::subscriptions::SubscriptionData;
use stream_log_shared::messages::user::{SelfUserData, UpdateUser};

pub async fn handle_profile_update(
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	user: &SelfUserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_data: UpdateUser,
) -> Result<(), HandleConnectionError> {
	let red: i32 = update_data.color.r.into();
	let green: i32 = update_data.color.g.into();
	let blue: i32 = update_data.color.b.into();

	let update_result = {
		let mut db_connection = match db_connection_pool.get() {
			Ok(connection) => connection,
			Err(error) => {
				tide::log::error!(
					"A database connection error occurred updating a user profile: {}",
					error
				);
				return Ok(());
			}
		};
		diesel::update(users::table.filter(users::id.eq(&user.id)))
			.set((
				users::color_red.eq(red),
				users::color_green.eq(green),
				users::color_blue.eq(blue),
				users::use_spell_check.eq(update_data.use_spell_check),
			))
			.execute(&mut *db_connection)
	};
	if let Err(error) = update_result {
		tide::log::error!("Database error updating a user profile: {}", error);
		return Err(HandleConnectionError::ConnectionClosed);
	}

	let mut subscription_manager = subscription_manager.lock().await;
	let mut new_user = user.clone();
	new_user.color = update_data.color;
	new_user.use_spell_check = update_data.use_spell_check;

	let user_update = UserDataUpdate::User(new_user.clone());
	subscription_manager.send_message_to_user(&user.id, user_update).await;

	let admin_message = SubscriptionData::AdminUsersUpdate(new_user);
	let send_result = subscription_manager.broadcast_admin_user_message(admin_message).await;
	if let Err(error) = send_result {
		tide::log::error!("Failed to send user update to admin subscriptions: {}", error);
	}

	Ok(())
}
