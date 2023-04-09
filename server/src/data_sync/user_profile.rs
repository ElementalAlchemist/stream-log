use super::HandleConnectionError;
use crate::schema::users;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use stream_log_shared::messages::user::{UpdateUser, UserData};

pub async fn handle_profile_update(
	db_connection: Arc<Mutex<PgConnection>>,
	user: &UserData,
	update_data: UpdateUser,
) -> Result<(), HandleConnectionError> {
	let mut db_connection = db_connection.lock().await;
	match update_data {
		UpdateUser::UpdateColor(new_color) => {
			let red: i32 = new_color.r.into();
			let green: i32 = new_color.g.into();
			let blue: i32 = new_color.b.into();

			let update_result = diesel::update(users::table.filter(users::id.eq(&user.id)))
				.set((
					users::color_red.eq(red),
					users::color_green.eq(green),
					users::color_blue.eq(blue),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating user color: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
	}

	Ok(())
}
