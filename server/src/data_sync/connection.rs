use super::admin::handle_admin;
use super::event_selection::send_events;
use super::register::register_user;
use super::subscriptions::{subscribe_to_event, unsubscribe_all};
use super::user_profile::handle_profile_update;
use super::HandleConnectionError;
use crate::models::User;
use crate::schema::users;
use crate::synchronization::SubscriptionManager;
use crate::websocket_msg::recv_msg;
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use rgb::RGB8;
use stream_log_shared::messages::initial::{InitialMessage, UserDataLoad};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::RequestMessage;
use tide::Request;
use tide_openidconnect::OpenIdConnectRequestExt;
use tide_websockets::WebSocketConnection;

/// Runs the WebSocket connection with the user
pub async fn handle_connection(
	db_connection: Arc<Mutex<PgConnection>>,
	request: Request<()>,
	stream: WebSocketConnection,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> tide::Result<()> {
	let Some(openid_user_id) = request.user_id() else {
		let message = InitialMessage::new(UserDataLoad::MissingId);
		stream.send_json(&message).await?;
		return Ok(());
	};

	let results = {
		let mut db_connection = db_connection.lock().await;
		users::table
			.filter(users::openid_user_id.eq(&openid_user_id))
			.load::<User>(&mut *db_connection)
	};

	let user = match results {
		Ok(mut users) => {
			if users.len() > 1 {
				tide::log::error!("Duplicate OpenID user ID in database: {}", openid_user_id);
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

	let stream = Arc::new(Mutex::new(stream));

	match user {
		Some(user) => {
			let color = RGB8::new(
				user.color_red.try_into().unwrap(),
				user.color_green.try_into().unwrap(),
				user.color_blue.try_into().unwrap(),
			);
			let user_data = UserData {
				id: user.id.clone(),
				username: user.name.clone(),
				is_admin: user.is_admin,
				color,
			};
			let message = InitialMessage::new(UserDataLoad::User(user_data));
			stream.lock().await.send_json(&message).await?;
			if let Err(HandleConnectionError::SendError(error)) = process_messages(
				Arc::clone(&db_connection),
				Arc::clone(&stream),
				&user,
				subscription_manager,
			)
			.await
			{
				return Err(error);
			}
		}
		None => {
			let message = InitialMessage::new(UserDataLoad::NewUser);
			stream.lock().await.send_json(&message).await?;
			let user = match register_user(Arc::clone(&db_connection), Arc::clone(&stream), &openid_user_id).await {
				Ok(user) => user,
				Err(HandleConnectionError::SendError(error)) => return Err(error),
				Err(_) => return Ok(()),
			};
			if user.is_admin {
				if let Err(HandleConnectionError::SendError(error)) = process_messages(
					Arc::clone(&db_connection),
					Arc::clone(&stream),
					&user,
					subscription_manager,
				)
				.await
				{
					return Err(error);
				}
			}
		}
	}

	Ok(())
}

/// Handles messages from a user throughout the connection
async fn process_messages(
	db_connection: Arc<Mutex<PgConnection>>,
	stream: Arc<Mutex<WebSocketConnection>>,
	user: &User,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	loop {
		let stream = Arc::clone(&stream);
		let incoming_msg = {
			let mut stream = stream.lock().await;
			match recv_msg(&mut stream).await {
				Ok(msg) => msg,
				Err(error) => {
					error.log();
					return Err(HandleConnectionError::ConnectionClosed);
				}
			}
		};
		let incoming_msg: RequestMessage = match serde_json::from_str(&incoming_msg) {
			Ok(msg) => msg,
			Err(error) => {
				tide::log::error!("Received an invalid request message: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		};

		match incoming_msg {
			RequestMessage::ListAvailableEvents => send_events(&db_connection, stream, user).await?,
			RequestMessage::SubscribeToEvent(event_id) => {
				subscribe_to_event(
					Arc::clone(&db_connection),
					stream,
					user,
					Arc::clone(&subscription_manager),
					&event_id,
				)
				.await?
			}
			RequestMessage::UnsubscribeAll => unsubscribe_all(stream, user).await,
			RequestMessage::EventSubscriptionUpdate(event, update_data) => todo!(),
			RequestMessage::Admin(action) => handle_admin(stream, Arc::clone(&db_connection), user, action).await?,
			RequestMessage::UpdateProfile(update_data) => {
				handle_profile_update(Arc::clone(&db_connection), user, update_data).await?
			}
		}
	}
}
