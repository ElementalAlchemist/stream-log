use super::register::register_user;
use crate::config::ConfigDocument;
use crate::models::User;
use crate::schema::users;
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use stream_log_shared::messages::initial::{InitialMessage, UserData, UserDataLoad};
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

		match register_user(Arc::clone(&db_connection), &mut stream, &google_user_id).await {
			Ok(user) => user,
			Err(_) => return Ok(()),
		}
	};

	Ok(())
}
