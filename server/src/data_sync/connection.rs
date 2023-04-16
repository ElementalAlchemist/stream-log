use super::register::{check_username, register_user};
use super::subscriptions::{handle_event_update, subscribe_to_event, unsubscribe_from_event};
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
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::user_register::UserRegistration;
use stream_log_shared::messages::FromClientMessage;
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
	let user_data = user.map(|user| {
		let color = RGB8::new(
			user.color_red.try_into().unwrap(),
			user.color_green.try_into().unwrap(),
			user.color_blue.try_into().unwrap(),
		);
		UserData {
			id: user.id.clone(),
			username: user.name.clone(),
			is_admin: user.is_admin,
			color,
		}
	});

	let stream = Arc::new(Mutex::new(stream));

	let initial_message = match user_data.as_ref() {
		Some(user) => InitialMessage::new(UserDataLoad::User(user.clone())),
		None => InitialMessage::new(UserDataLoad::NewUser),
	};
	stream.lock().await.send_json(&initial_message).await?;

	let process_messages_result = process_messages(
		Arc::clone(&db_connection),
		Arc::clone(&stream),
		user_data,
		Arc::clone(&subscription_manager),
		&openid_user_id,
	)
	.await;

	match process_messages_result {
		Err(HandleConnectionError::SendError(error)) => Err(error),
		_ => Ok(()),
	}
}

/// Handles messages from a user throughout the connection
async fn process_messages(
	db_connection: Arc<Mutex<PgConnection>>,
	stream: Arc<Mutex<WebSocketConnection>>,
	mut user: Option<UserData>,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	openid_user_id: &str,
) -> Result<(), HandleConnectionError> {
	let result = loop {
		if let Err(error) = process_message(
			&db_connection,
			&stream,
			&mut user,
			&subscription_manager,
			openid_user_id,
		)
		.await
		{
			break Err(error);
		}
	};

	if let Some(user) = user.as_ref() {
		subscription_manager.lock().await.unsubscribe_user_from_all(user).await;
	}

	result
}

async fn process_message(
	db_connection: &Arc<Mutex<PgConnection>>,
	stream: &Arc<Mutex<WebSocketConnection>>,
	user: &mut Option<UserData>,
	subscription_manager: &Arc<Mutex<SubscriptionManager>>,
	openid_user_id: &str,
) -> Result<(), HandleConnectionError> {
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
	let incoming_msg: FromClientMessage = match serde_json::from_str(&incoming_msg) {
		Ok(msg) => msg,
		Err(error) => {
			tide::log::error!("Received an invalid request message: {}", error);
			return Err(HandleConnectionError::ConnectionClosed);
		}
	};

	match incoming_msg {
		FromClientMessage::StartSubscription(subscription_type) => {
			let Some(user) = user.as_ref() else { return Ok(()); }; // Only logged-in users can subscribe
			match subscription_type {
				SubscriptionType::EventLogData(event_id) => {
					subscribe_to_event(
						Arc::clone(db_connection),
						Arc::clone(stream),
						user,
						Arc::clone(subscription_manager),
						&event_id,
					)
					.await?
				}
				SubscriptionType::AdminUsers => todo!(),
				SubscriptionType::AdminEvents => todo!(),
				SubscriptionType::AdminPermissionGroups => todo!(),
				SubscriptionType::AdminPermissionGroupEvents => todo!(),
				SubscriptionType::AdminPermissionGroupUsers => todo!(),
				SubscriptionType::AdminEntryTypes => todo!(),
				SubscriptionType::AdminEntryTypesEvents => todo!(),
				SubscriptionType::AdminTags => todo!(),
				SubscriptionType::AdminTagEvents => todo!(),
				SubscriptionType::AdminEventEditors => todo!(),
			}
		}
		FromClientMessage::EndSubscription(subscription_type) => {
			let Some(user) = user.as_ref() else { return Ok(()); }; // Users who aren't logged in can't be subscribed
			match subscription_type {
				SubscriptionType::EventLogData(event_id) => {
					unsubscribe_from_event(Arc::clone(subscription_manager), user, &event_id).await?
				}
				SubscriptionType::AdminUsers => todo!(),
				SubscriptionType::AdminEvents => todo!(),
				SubscriptionType::AdminPermissionGroups => todo!(),
				SubscriptionType::AdminPermissionGroupEvents => todo!(),
				SubscriptionType::AdminPermissionGroupUsers => todo!(),
				SubscriptionType::AdminEntryTypes => todo!(),
				SubscriptionType::AdminEntryTypesEvents => todo!(),
				SubscriptionType::AdminTags => todo!(),
				SubscriptionType::AdminTagEvents => todo!(),
				SubscriptionType::AdminEventEditors => todo!(),
			}
		}
		FromClientMessage::SubscriptionMessage(subscription_update) => {
			let Some(user) = user.as_ref() else { return Ok(()); }; // One must be subscribed (and therefore logged in) to send a subscription update message
			match *subscription_update {
				SubscriptionTargetUpdate::EventUpdate(event, update_data) => {
					handle_event_update(
						Arc::clone(db_connection),
						Arc::clone(subscription_manager),
						&event,
						user,
						update_data,
					)
					.await?
				}
				SubscriptionTargetUpdate::AdminEventsUpdate(update_data) => todo!(),
				SubscriptionTargetUpdate::AdminEntryTypesUpdate(update_data) => todo!(),
				SubscriptionTargetUpdate::AdminEntryTypesEventsUpdate(update_data) => todo!(),
				SubscriptionTargetUpdate::AdminPermissionGroupsUpdate(update_data) => todo!(),
				SubscriptionTargetUpdate::AdminTagsUpdate(update_data) => todo!(),
				SubscriptionTargetUpdate::AdminUserUpdate(user) => todo!(),
				SubscriptionTargetUpdate::AdminEventEditorsUpdate(update_data) => todo!(),
				SubscriptionTargetUpdate::AdminUserPermissionGroupsUpdate(update_data) => todo!(),
			}
		}
		FromClientMessage::RegistrationRequest(registration_data) => {
			if user.is_none() {
				match registration_data {
					UserRegistration::CheckUsername(username) => {
						check_username(Arc::clone(db_connection), Arc::clone(stream), &username).await?
					}
					UserRegistration::Finalize(registration_data) => {
						register_user(
							Arc::clone(db_connection),
							Arc::clone(stream),
							openid_user_id,
							registration_data,
							user,
						)
						.await?
					}
				}
			}
		}
		FromClientMessage::UpdateProfile(profile_data) => {
			if let Some(user) = user.as_ref() {
				handle_profile_update(Arc::clone(db_connection), user, profile_data).await?;
			}
		}
	}

	Ok(())
}
