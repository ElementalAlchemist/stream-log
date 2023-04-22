use super::register::{check_username, register_user};
use super::subscriptions::{handle_event_update, subscribe_to_event, unsubscribe_from_event};
use super::user_profile::handle_profile_update;
use super::HandleConnectionError;
use crate::data_sync::{SubscriptionManager, UserDataUpdate};
use crate::models::{Permission, User};
use crate::schema::users;
use crate::websocket_msg::recv_msg;
use async_std::channel::{unbounded, Receiver, Sender};
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use erased_serde::Serialize;
use futures::{select, FutureExt};
use rgb::RGB8;
use std::collections::HashMap;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::initial::{InitialMessage, UserDataLoad};
use stream_log_shared::messages::subscriptions::{SubscriptionData, SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::{UserData, UserSubscriptionUpdate};
use stream_log_shared::messages::user_register::UserRegistration;
use stream_log_shared::messages::{FromClientMessage, FromServerMessage};
use tide::Request;
use tide_openidconnect::OpenIdConnectRequestExt;
use tide_websockets::WebSocketConnection;

pub enum ConnectionUpdate {
	SendData(Box<dyn Serialize + Send + Sync>),
	UserUpdate(UserDataUpdate),
}

/// Runs the WebSocket connection with the user
pub async fn handle_connection(
	db_connection: Arc<Mutex<PgConnection>>,
	request: Request<()>,
	mut stream: WebSocketConnection,
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

	let initial_message = match user_data.as_ref() {
		Some(user) => InitialMessage::new(UserDataLoad::User(user.clone())),
		None => InitialMessage::new(UserDataLoad::NewUser),
	};
	stream.send_json(&initial_message).await?;

	let process_messages_result = process_messages(
		Arc::clone(&db_connection),
		&mut stream,
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
	stream: &mut WebSocketConnection,
	mut user: Option<UserData>,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	openid_user_id: &str,
) -> Result<(), HandleConnectionError> {
	let mut event_permission_cache: HashMap<Event, Option<Permission>> = HashMap::new();
	let (conn_update_tx, conn_update_rx) = unbounded::<ConnectionUpdate>();
	let result = loop {
		let args = ProcessMessageParams {
			db_connection: &db_connection,
			stream,
			user: &mut user,
			subscription_manager: &subscription_manager,
			openid_user_id,
			event_permission_cache: &mut event_permission_cache,
			conn_update_tx: conn_update_tx.clone(),
			conn_update_rx: &conn_update_rx,
		};
		if let Err(error) = process_message(args).await {
			break Err(error);
		}
	};

	if let Some(user) = user.as_ref() {
		subscription_manager.lock().await.unsubscribe_user_from_all(user).await;
	}

	result
}

struct ProcessMessageParams<'a> {
	db_connection: &'a Arc<Mutex<PgConnection>>,
	stream: &'a mut WebSocketConnection,
	user: &'a mut Option<UserData>,
	subscription_manager: &'a Arc<Mutex<SubscriptionManager>>,
	openid_user_id: &'a str,
	event_permission_cache: &'a mut HashMap<Event, Option<Permission>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	conn_update_rx: &'a Receiver<ConnectionUpdate>,
}

async fn process_message(args: ProcessMessageParams<'_>) -> Result<(), HandleConnectionError> {
	let mut message_to_send: Option<Box<dyn Serialize + Send + Sync>> = None;

	{
		let mut conn_update_future = args.conn_update_rx.recv().fuse();
		let mut recv_msg_future = Box::pin(recv_msg(args.stream).fuse());
		select! {
			conn_update_result = conn_update_future => {
				match conn_update_result {
					Ok(conn_update) => match conn_update {
						ConnectionUpdate::SendData(send_message) => {
							message_to_send = Some(send_message);
						}
						ConnectionUpdate::UserUpdate(user_data_update) => {
							match user_data_update {
								UserDataUpdate::User(new_user_data) => *args.user = Some(new_user_data),
								UserDataUpdate::EventPermissions(event, new_permission) => { args.event_permission_cache.insert(event, new_permission); }
							}
							if let Some(user) = args.user.clone() {
								let available_events: Vec<Event> = args.event_permission_cache.iter().filter(|(_, permission)| permission.is_some()).map(|(event, _)| event.clone()).collect();
								let user_subscription_data = UserSubscriptionUpdate { user, available_events };
								let message = FromServerMessage::SubscriptionMessage(Box::new(SubscriptionData::UserUpdate(user_subscription_data)));
								message_to_send = Some(Box::new(message));
							}
						}
					}
					Err(_) => return Err(HandleConnectionError::ConnectionClosed)
				}
			}
			recv_msg_result = recv_msg_future => {
				let incoming_msg = {
					match recv_msg_result {
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
						let Some(user) = args.user.as_ref() else { return Ok(()); }; // Only logged-in users can subscribe
						match subscription_type {
							SubscriptionType::EventLogData(event_id) => {
								subscribe_to_event(
									Arc::clone(args.db_connection),
									args.conn_update_tx,
									user,
									Arc::clone(args.subscription_manager),
									&event_id,
									args.event_permission_cache,
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
							SubscriptionType::AdminEventEditors => todo!(),
						}
					}
					FromClientMessage::EndSubscription(subscription_type) => {
						let Some(user) = args.user.as_ref() else { return Ok(()); }; // Users who aren't logged in can't be subscribed
						match subscription_type {
							SubscriptionType::EventLogData(event_id) => {
								unsubscribe_from_event(Arc::clone(args.subscription_manager), user, &event_id).await?
							}
							SubscriptionType::AdminUsers => todo!(),
							SubscriptionType::AdminEvents => todo!(),
							SubscriptionType::AdminPermissionGroups => todo!(),
							SubscriptionType::AdminPermissionGroupEvents => todo!(),
							SubscriptionType::AdminPermissionGroupUsers => todo!(),
							SubscriptionType::AdminEntryTypes => todo!(),
							SubscriptionType::AdminEntryTypesEvents => todo!(),
							SubscriptionType::AdminTags => todo!(),
							SubscriptionType::AdminEventEditors => todo!(),
						}
					}
					FromClientMessage::SubscriptionMessage(subscription_update) => {
						let Some(user) = args.user.as_ref() else { return Ok(()); }; // One must be subscribed (and therefore logged in) to send a subscription update message
						match *subscription_update {
							SubscriptionTargetUpdate::EventUpdate(event, update_data) => {
								handle_event_update(
									Arc::clone(args.db_connection),
									Arc::clone(args.subscription_manager),
									&event,
									user,
									args.event_permission_cache,
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
						if args.user.is_none() {
							match registration_data {
								UserRegistration::CheckUsername(username) => {
									check_username(Arc::clone(args.db_connection), args.conn_update_tx, &username).await?
								}
								UserRegistration::Finalize(registration_data) => {
									register_user(
										Arc::clone(args.db_connection),
										args.conn_update_tx,
										args.openid_user_id,
										registration_data,
										args.user,
									)
									.await?
								}
							}
						}
					}
					FromClientMessage::UpdateProfile(profile_data) => {
						if let Some(user) = args.user.as_ref() {
							handle_profile_update(Arc::clone(args.db_connection), user, profile_data).await?;
						}
					}
				}
			}
		}
	}

	if let Some(message) = message_to_send {
		args.stream.send_json(&message).await?;
	}
	Ok(())
}
