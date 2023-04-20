use super::connection::ConnectionUpdate;
use super::HandleConnectionError;
use crate::models::User;
use crate::schema::users;
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use rgb::RGB8;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::user_register::{
	RegistrationFinalizeResponse, RegistrationResponse, UserRegistrationFinalize, UsernameCheckResponse,
	USERNAME_LENGTH_LIMIT,
};
use stream_log_shared::messages::FromServerMessage;

/// Checks whether the username being queried is already registered
pub async fn check_username(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	username: &str,
) -> Result<(), HandleConnectionError> {
	if username.len() > USERNAME_LENGTH_LIMIT {
		let response =
			FromServerMessage::RegistrationResponse(RegistrationResponse::UsernameCheck(UsernameCheckResponse {
				username: username.to_string(),
				available: false,
			}));
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(response)))
			.await?;
		return Ok(());
	}
	let check_results: QueryResult<Vec<User>> = {
		let mut db_connection = db_connection.lock().await;
		users::table.filter(users::name.eq(username)).load(&mut *db_connection)
	};
	if let Ok(data) = check_results {
		let available = data.is_empty();
		let response =
			FromServerMessage::RegistrationResponse(RegistrationResponse::UsernameCheck(UsernameCheckResponse {
				username: username.to_string(),
				available,
			}));
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(response)))
			.await?;
	}
	Ok(())
}

/// Registers the user if the registration is valid
pub async fn register_user(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	openid_user_id: &str,
	registration_data: UserRegistrationFinalize,
	user: &mut Option<UserData>,
) -> Result<(), HandleConnectionError> {
	let response = if registration_data.name.is_empty() {
		FromServerMessage::RegistrationResponse(RegistrationResponse::Finalize(
			RegistrationFinalizeResponse::NoUsernameSpecified,
		))
	} else if registration_data.name.len() > USERNAME_LENGTH_LIMIT {
		FromServerMessage::RegistrationResponse(RegistrationResponse::Finalize(
			RegistrationFinalizeResponse::UsernameTooLong,
		))
	} else {
		let new_user_id = cuid2::create_id();
		let color_red: i32 = registration_data.color.r.into();
		let color_green: i32 = registration_data.color.g.into();
		let color_blue: i32 = registration_data.color.b.into();

		let registration_result: QueryResult<User> = {
			let mut db_connection = db_connection.lock().await;
			db_connection.transaction(|db_connection| {
				let initial_user_check: Vec<String> = users::table.select(users::id).limit(1).load(db_connection)?;
				let has_users = !initial_user_check.is_empty();

				// If this is the first account, it should be an administrator account so that there can be an administrator
				// (without manually setting the database directly). Otherwise, users should require approval.
				// This is for the first account, so if something goes wrong, the database can be wiped and started over with no
				// problem.
				let new_user = User {
					id: new_user_id,
					openid_user_id: openid_user_id.to_owned(),
					name: registration_data.name,
					is_admin: !has_users,
					color_red,
					color_green,
					color_blue,
				};

				let user_record: User = diesel::insert_into(users::table)
					.values(&new_user)
					.get_result(db_connection)?;
				Ok(user_record)
			})
		};

		match registration_result {
			Ok(new_user) => {
				let color = RGB8::new(
					color_red.try_into().unwrap(),
					color_green.try_into().unwrap(),
					color_blue.try_into().unwrap(),
				);
				let user_data = UserData {
					id: new_user.id.clone(),
					username: new_user.name.clone(),
					is_admin: new_user.is_admin,
					color,
				};
				*user = Some(user_data.clone());
				FromServerMessage::RegistrationResponse(RegistrationResponse::Finalize(
					RegistrationFinalizeResponse::Success(user_data),
				))
			}
			Err(error) => {
				if let diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, error_info) = &error {
					if error_info.constraint_name() == Some("users_name_key") {
						FromServerMessage::RegistrationResponse(RegistrationResponse::Finalize(
							RegistrationFinalizeResponse::UsernameInUse,
						))
					} else {
						tide::log::error!("Database error: {}", error);
						return Ok(());
					}
				} else {
					tide::log::error!("Database error: {}", error);
					return Ok(());
				}
			}
		}
	};

	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(response)))
		.await?;
	Ok(())
}
