use crate::config::ConfigDocument;
use crate::models::User;
use crate::schema::users;
use async_std::stream::StreamExt;
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use stream_log_shared::messages::user::{UserData, UserDataLoad};
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
		stream.send_json(&UserDataLoad::MissingId).await?;
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
				stream.send_json(&UserDataLoad::Error).await?;
				return Ok(());
			} else {
				users.pop()
			}
		}
		Err(error) => {
			tide::log::error!("Failed to retrive user data from database: {}", error);
			stream.send_json(&UserDataLoad::Error).await?;
			return Ok(());
		}
	};

	if let Some(user) = user {
		let user_data = UserData {
			id: user.id,
			username: user.name,
		};
		stream.send_json(&UserDataLoad::User(user_data)).await?;
	} else {
		stream.send_json(&UserDataLoad::NewUser).await?;
	}

	let response = stream.next().await;
	tide::log::info!("{:?}", response);

	Ok(())
}
