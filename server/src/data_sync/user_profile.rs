use super::{HandleConnectionError, SubscriptionManager};
use crate::schema::users;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use stream_log_shared::messages::subscriptions::SubscriptionData;
use stream_log_shared::messages::user::{UpdateUser, UserData};

pub async fn handle_profile_update(
	db_connection: Arc<Mutex<PgConnection>>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_data: UpdateUser,
) -> Result<(), HandleConnectionError> {
	match update_data {
		UpdateUser::UpdateColor(new_color) => {
			let red: i32 = new_color.r.into();
			let green: i32 = new_color.g.into();
			let blue: i32 = new_color.b.into();

			let update_result = {
				let mut db_connection = db_connection.lock().await;
				diesel::update(users::table.filter(users::id.eq(&user.id)))
					.set((
						users::color_red.eq(red),
						users::color_green.eq(green),
						users::color_blue.eq(blue),
					))
					.execute(&mut *db_connection)
			};
			if let Err(error) = update_result {
				tide::log::error!("Database error updating user color: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}

			let subscription_manager = subscription_manager.lock().await;
			let mut new_user = user.clone();
			new_user.color = new_color;
			let admin_message = SubscriptionData::AdminUsersUpdate(new_user);
			let send_result = subscription_manager.broadcast_admin_user_message(admin_message).await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send user update to admin subscriptions: {}", error);
			}
		}
	}

	Ok(())
}
