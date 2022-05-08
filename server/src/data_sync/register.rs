use super::HandleConnectionError;
use crate::models::{DefaultRole, Role, User};
use crate::schema::{default_roles, roles, users};
use crate::websocket_msg::recv_msg;
use async_std::sync::{Arc, Mutex};
use cuid::cuid;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::user_register::{
	RegistrationResponse, UserRegistration, UsernameCheckResponse, UsernameCheckStatus,
};
use stream_log_shared::messages::{DataError, DataMessage};
use tide_websockets::WebSocketConnection;

/// Runs the user registration portion of the connection
pub async fn register_user(
	db_connection: Arc<Mutex<PgConnection>>,
	stream: &mut WebSocketConnection,
	google_user_id: &str,
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
				if username.len() > 64 {
					let response = UsernameCheckResponse {
						username,
						status: UsernameCheckStatus::Unavailable,
					};
					stream.send_json(&response).await?;
					continue;
				}
				let check_results: QueryResult<Vec<User>> = {
					let db_connection = db_connection.lock().await;
					users::table.filter(users::name.eq(&username)).load(&*db_connection)
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
				if data.name.len() > 64 {
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
				let new_user = User {
					id: new_user_id,
					google_user_id: google_user_id.to_owned(),
					name: data.name,
				};
				let user_result: QueryResult<User> = {
					let db_connection = db_connection.lock().await;
					db_connection.transaction(|| {
						let mut default_roles: Vec<DefaultRole> = default_roles::table.load(&*db_connection)?;
						let user_record: User = diesel::insert_into(users::table)
							.values(&new_user)
							.get_result(&*db_connection)?;
						let roles: Vec<Role> = default_roles
							.drain(..)
							.map(|default_role| Role {
								user_id: user_record.id.clone(),
								event: default_role.event,
								permission_level: default_role.permission_level,
							})
							.collect();
						diesel::insert_into(roles::table)
							.values(&roles)
							.execute(&*db_connection)?;
						Ok(user_record)
					})
				};
				match user_result {
					Ok(data) => {
						let user_data = UserData {
							id: data.id.clone(),
							username: data.name.clone(),
						};
						let response_message = DataMessage::Ok(RegistrationResponse::Success(user_data));
						stream.send_json(&response_message).await?;
						break Ok(data);
					}
					Err(error) => {
						if let diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, error_info) =
							&error
						{
							if error_info.column_name() == Some("name") {
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
