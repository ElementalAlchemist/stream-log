use crate::config::ConfigDocument;
use crate::models::User;
use crate::schema::users;
use async_std::stream::StreamExt;
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use stream_log_shared::messages::initial::{InitialMessage, UserData, UserDataLoad};
use tide::Request;
use tide_openidconnect::OpenIdConnectRequestExt;
use tide_websockets::WebSocketConnection;

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

	let message = if let Some(user) = user {
		let user_data = UserData {
			id: user.id,
			username: user.name,
		};
		InitialMessage::new(UserDataLoad::User(user_data))
	} else {
		InitialMessage::new(UserDataLoad::NewUser)
	};
	stream.send_json(&message).await?;

	let response = stream.next().await;
	tide::log::info!("{:?}", response);

	Ok(())
}
