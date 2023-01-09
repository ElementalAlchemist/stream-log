use super::HandleConnectionError;
use crate::models::User;
use crate::schema::users;
use crate::websocket_msg::recv_msg;
use async_std::sync::{Arc, Mutex};
use cuid::cuid;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use rgb::RGB8;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::user_register::{
	RegistrationResponse, UserRegistration, UsernameCheckResponse, UsernameCheckStatus, USERNAME_LENGTH_LIMIT,
};
use stream_log_shared::messages::{DataError, DataMessage};
use tide_websockets::WebSocketConnection;

/// Runs the user registration portion of the connection
pub async fn register_user(
	db_connection: Arc<Mutex<PgConnection>>,
	stream: &mut WebSocketConnection,
	openid_user_id: &str,
) -> Result<User, HandleConnectionError> {
	loop {
		let response = match recv_msg(stream).await {
			Ok(resp) => resp,
			Err(error) => {
				error.log();
				return Err(HandleConnectionError::ConnectionClosed);
			}
		};
		let registration_data: UserRegistration = match serde_json::from_str(&response) {
			Ok(val) => val,
			Err(error) => {
				tide::log::error!("Received an incorrect message during user registration: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		};
		match registration_data {
			UserRegistration::CheckUsername(username) => {
				if username.len() > USERNAME_LENGTH_LIMIT {
					let response = DataMessage::Ok(UsernameCheckResponse {
						username,
						status: UsernameCheckStatus::Unavailable,
					});
					stream.send_json(&response).await?;
					continue;
				}
				let check_results: QueryResult<Vec<User>> = {
					let mut db_connection = db_connection.lock().await;
					users::table.filter(users::name.eq(&username)).load(&mut *db_connection)
				};
				let message = match check_results {
					Ok(data) => {
						let status = if data.is_empty() {
							UsernameCheckStatus::Available
						} else {
							UsernameCheckStatus::Unavailable
						};
						DataMessage::Ok(UsernameCheckResponse { username, status })
					}
					Err(error) => {
						tide::log::error!("Database error: {}", error);
						DataMessage::Err(DataError::DatabaseError)
					}
				};
				stream.send_json(&message).await?;
			}
			UserRegistration::Finalize(data) => {
				if data.name.is_empty() {
					let response_message: DataMessage<RegistrationResponse> =
						DataMessage::Ok(RegistrationResponse::NoUsernameSpecified);
					stream.send_json(&response_message).await?;
					continue;
				}
				if data.name.len() > USERNAME_LENGTH_LIMIT {
					let response_message: DataMessage<RegistrationResponse> =
						DataMessage::Ok(RegistrationResponse::UsernameTooLong);
					stream.send_json(&response_message).await?;
					continue;
				}
				let new_user_id = match cuid() {
					Ok(id) => id,
					Err(error) => {
						tide::log::error!("CUID generation error: {}", error);
						let response_message: DataMessage<RegistrationResponse> =
							DataMessage::Err(DataError::ServerError);
						stream.send_json(&response_message).await?;
						continue;
					}
				};
				let color_red: i32 = data.color.r.into();
				let color_green: i32 = data.color.g.into();
				let color_blue: i32 = data.color.b.into();

				let user_result: QueryResult<User> = {
					let mut db_connection = db_connection.lock().await;
					db_connection.transaction(|db_connection| {
						let initial_user_check: Vec<String> =
							users::table.select(users::id).limit(1).load(db_connection)?;
						let has_users = !initial_user_check.is_empty();

						// If this is the first account, it should be an administrator account so that there can be an administrator
						// (without manually setting the database directly). Otherwise, users should require approval.
						// This is for the first account, so if something goes wrong, the database can be wiped and started over with no
						// problem.
						let new_user = User {
							id: new_user_id,
							openid_user_id: openid_user_id.to_owned(),
							name: data.name,
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
				match user_result {
					Ok(data) => {
						let color = RGB8::new(
							color_red.try_into().unwrap(),
							color_green.try_into().unwrap(),
							color_blue.try_into().unwrap(),
						);
						let user_data = UserData {
							id: data.id.clone(),
							username: data.name.clone(),
							is_admin: data.is_admin,
							color,
						};
						let response_message = DataMessage::Ok(RegistrationResponse::Success(user_data));
						stream.send_json(&response_message).await?;
						break Ok(data);
					}
					Err(error) => {
						if let diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, error_info) =
							&error
						{
							if error_info.constraint_name() == Some("users_name_key") {
								let response_message = DataMessage::Ok(RegistrationResponse::UsernameInUse);
								stream.send_json(&response_message).await?;
								continue;
							}
						}
						tide::log::error!("Database error: {}", error);
						let response_message: DataMessage<RegistrationResponse> =
							DataMessage::Err(DataError::DatabaseError);
						stream.send_json(&response_message).await?;
						continue;
					}
				}
			}
		}
	}
}
