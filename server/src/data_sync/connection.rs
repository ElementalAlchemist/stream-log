use crate::config::ConfigDocument;
use crate::models::User;
use crate::schema::users;
use crate::websocket_msg::recv_msg;
use async_std::sync::{Arc, Mutex};
use cuid::cuid;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use stream_log_shared::messages::initial::{InitialMessage, UserData, UserDataLoad};
use stream_log_shared::messages::user_register::{
	RegistrationResponse, UserRegistration, UsernameCheckResponse, UsernameCheckStatus,
};
use stream_log_shared::messages::DataMessage;
use tide::Request;
use tide_openidconnect::OpenIdConnectRequestExt;
use tide_websockets::WebSocketConnection;

/// Runs the WebSocket connection with the user
pub async fn handle_connection(
	config: Arc<ConfigDocument>,
	db_connection: Arc<Mutex<PgConnection>>,
	request: Request<()>,
	mut stream: WebSocketConnection,
) -> tide::Result<()> {
	let google_user_id = if let Some(id) = request.user_id() {
		id
	} else {
		let message = InitialMessage::new(UserDataLoad::MissingId);
		stream.send_json(&message).await?;
		return Ok(());
	};

	let results = {
		let connection = db_connection.lock().await;
		users::table
			.filter(users::google_user_id.eq(&google_user_id))
			.load::<User>(&*connection)
	};

	let user = match results {
		Ok(mut users) => {
			if users.len() > 1 {
				tide::log::error!("Duplicate Google user ID in database: {}", google_user_id);
				let message = InitialMessage::new(UserDataLoad::Error);
				stream.send_json(&message).await?;
				return Ok(());
			} else {
				users.pop()
			}
		}
		Err(error) => {
			tide::log::error!("Failed to retrive user data from database: {}", error);
			let message = InitialMessage::new(UserDataLoad::Error);
			stream.send_json(&message).await?;
			return Ok(());
		}
	};

	let user = if let Some(user) = user {
		let user_data = UserData {
			id: user.id.clone(),
			username: user.name.clone(),
		};
		stream
			.send_json(&InitialMessage::new(UserDataLoad::User(user_data)))
			.await?;
		user
	} else {
		stream.send_json(&InitialMessage::new(UserDataLoad::NewUser)).await?;

		loop {
			let response = match recv_msg(&mut stream).await {
				Ok(resp) => resp,
				Err(error) => {
					error.log();
					return Ok(());
				}
			};
			let registration_data: UserRegistration = match serde_json::from_str(&response) {
				Ok(val) => val,
				Err(error) => {
					tide::log::error!("Received an incorrect message during user registration: {}", error);
					return Ok(());
				}
			};
			match registration_data {
				UserRegistration::CheckUsername(username) => {
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
							DataMessage::Message(UsernameCheckResponse { username, status })
						}
						Err(error) => {
							tide::log::error!("Database error: {}", error);
							DataMessage::DatabaseError
						}
					};
					stream.send_json(&message).await?;
				}
				UserRegistration::Finalize(data) => {
					let new_user_id = match cuid() {
						Ok(id) => id,
						Err(error) => {
							tide::log::error!("CUID generation error: {}", error);
							let response_message: DataMessage<RegistrationResponse> = DataMessage::ServerError;
							stream.send_json(&response_message).await?;
							continue;
						}
					};
					let new_user = User {
						id: new_user_id,
						google_user_id: google_user_id.clone(),
						name: data.name,
					};
					let insert_result: QueryResult<User> = {
						let db_connection = db_connection.lock().await;
						diesel::insert_into(users::table)
							.values(&new_user)
							.get_result(&*db_connection)
					};
					match insert_result {
						Ok(data) => {
							let response_message = DataMessage::Message(RegistrationResponse::Success);
							stream.send_json(&response_message).await?;
							break data;
						}
						Err(error) => {
							if let diesel::result::Error::DatabaseError(
								DatabaseErrorKind::UniqueViolation,
								error_info,
							) = &error
							{
								if error_info.column_name() == Some("name") {
									let response_message = DataMessage::Message(RegistrationResponse::UsernameInUse);
									stream.send_json(&response_message).await?;
									continue;
								}
							}
							tide::log::error!("Database error: {}", error);
							let response_message: DataMessage<RegistrationResponse> = DataMessage::DatabaseError;
							stream.send_json(&response_message).await?;
							continue;
						}
					}
				}
			}
		}
	};

	Ok(())
}
