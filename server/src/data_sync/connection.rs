// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::register::{check_username, register_user};
use super::subscriptions::admin_applications::{handle_admin_applications_message, subscribe_to_admin_applications};
use super::subscriptions::admin_editors::{handle_admin_editors_message, subscribe_to_admin_editors};
use super::subscriptions::admin_entry_types::{
	handle_admin_entry_type_event_message, handle_admin_entry_type_message, subscribe_to_admin_entry_types,
	subscribe_to_admin_entry_types_events,
};
use super::subscriptions::admin_events::{handle_admin_event_message, subscribe_to_admin_events};
use super::subscriptions::admin_pages::{handle_admin_info_pages_message, subscribe_to_admin_info_pages};
use super::subscriptions::admin_permission_groups::{
	handle_admin_permission_group_users_message, handle_admin_permission_groups_message,
	subscribe_to_admin_permission_groups, subscribe_to_admin_permission_groups_users,
};
use super::subscriptions::admin_tabs::{handle_admin_event_log_tabs_message, subscribe_to_admin_event_log_tabs};
use super::subscriptions::admin_users::{handle_admin_users_message, subscribe_to_admin_users};
use super::subscriptions::events::{handle_event_update, subscribe_to_event};
use super::user_profile::handle_profile_update;
use super::HandleConnectionError;
use crate::data_sync::{SubscriptionManager, UserDataUpdate};
use crate::models::{Event as EventDb, Permission, PermissionEvent, User};
use crate::schema::{events, permission_events, user_permissions, users};
use crate::websocket_msg::{recv_msg, WebSocketRecvError};
use async_std::channel::{unbounded, Receiver, RecvError, Sender};
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

	let mut db_conn = db_connection.lock().await;
	let results: QueryResult<Vec<User>> = users::table
		.filter(users::openid_user_id.eq(&openid_user_id))
		.load(&mut *db_conn);

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
			tide::log::error!("Failed to retrieve user data from database: {}", error);
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

	let event_permission_cache: HashMap<Event, Option<Permission>> = if let Some(user) = user_data.as_ref() {
		let permission_events: QueryResult<Vec<PermissionEvent>> = permission_events::table
			.filter(
				permission_events::permission_group.eq_any(
					user_permissions::table
						.filter(user_permissions::user_id.eq(&user.id))
						.select(user_permissions::permission_group),
				),
			)
			.load(&mut *db_conn);
		let permission_events = match permission_events {
			Ok(permission_events) => permission_events,
			Err(error) => {
				tide::log::error!("Failed to retrieve available events from database: {}", error);
				let message = InitialMessage::new(UserDataLoad::Error);
				stream.send_json(&message).await?;
				return Ok(());
			}
		};
		let event_ids: Vec<String> = permission_events
			.iter()
			.map(|permission_event| permission_event.event.clone())
			.collect();
		let events: Vec<EventDb> = match events::table.filter(events::id.eq_any(&event_ids)).load(&mut *db_conn) {
			Ok(events) => events,
			Err(error) => {
				tide::log::error!("Failed to retrieve events from database: {}", error);
				let message = InitialMessage::new(UserDataLoad::Error);
				stream.send_json(&message).await?;
				return Ok(());
			}
		};
		let events: HashMap<String, Event> = events
			.into_iter()
			.map(|event| (event.id.clone(), event.into()))
			.collect();
		let mut available_events: HashMap<Event, Option<Permission>> = HashMap::new();
		for permission_event in permission_events {
			// We can expect the events we found to remain in the database, as nothing should remove them.
			let event = events.get(&permission_event.event).unwrap().clone();
			available_events.insert(event, Some(permission_event.level));
		}
		available_events
	} else {
		HashMap::new()
	};

	drop(db_conn);

	let initial_message = match user_data.as_ref() {
		Some(user) => {
			let available_events: Vec<Event> = event_permission_cache
				.iter()
				.filter(|(_, permission)| permission.is_some())
				.map(|(event, _)| event.clone())
				.collect();
			InitialMessage::new(UserDataLoad::User(user.clone(), available_events))
		}
		None => InitialMessage::new(UserDataLoad::NewUser),
	};
	stream.send_json(&initial_message).await?;

	let process_messages_result = process_messages(
		Arc::clone(&db_connection),
		&mut stream,
		user_data,
		Arc::clone(&subscription_manager),
		&openid_user_id,
		event_permission_cache,
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
	mut event_permission_cache: HashMap<Event, Option<Permission>>,
) -> Result<(), HandleConnectionError> {
	let (conn_update_tx, conn_update_rx) = unbounded::<ConnectionUpdate>();
	let connection_id = cuid2::create_id();

	if let Some(user) = user.as_ref() {
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager
			.subscribe_to_self_user(&connection_id, user, conn_update_tx.clone())
			.await;
	}

	let result = loop {
		let args = ProcessMessageParams {
			db_connection: &db_connection,
			stream,
			user: &mut user,
			connection_id: &connection_id,
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

	subscription_manager
		.lock()
		.await
		.unsubscribe_from_all(&connection_id)
		.await?;

	result
}

struct ProcessMessageParams<'a> {
	db_connection: &'a Arc<Mutex<PgConnection>>,
	stream: &'a mut WebSocketConnection,
	user: &'a mut Option<UserData>,
	connection_id: &'a str,
	subscription_manager: &'a Arc<Mutex<SubscriptionManager>>,
	openid_user_id: &'a str,
	event_permission_cache: &'a mut HashMap<Event, Option<Permission>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	conn_update_rx: &'a Receiver<ConnectionUpdate>,
}

async fn process_message(args: ProcessMessageParams<'_>) -> Result<(), HandleConnectionError> {
	let process_result = {
		let mut conn_update_future = args.conn_update_rx.recv().fuse();
		let mut recv_msg_future = Box::pin(recv_msg(args.stream).fuse());
		select! {
			conn_update_result = conn_update_future => process_connection_update(conn_update_result, args.user, args.event_permission_cache),
			recv_msg_result = recv_msg_future => {
				let incoming_msg_params = ProcessIncomingMessageParams { recv_msg_result, db_connection: args.db_connection, conn_update_tx: args.conn_update_tx, user: args.user, connection_id: args.connection_id, subscription_manager: args.subscription_manager, openid_user_id: args.openid_user_id, event_permission_cache: args.event_permission_cache };
				match process_incoming_message(incoming_msg_params).await {
					Ok(_) => Ok(None),
					Err(error) => Err(error)
				}
			}
		}
	};

	match process_result {
		Ok(message) => {
			if let Some(message_to_send) = message {
				args.stream.send_json(&message_to_send).await?;
			}
			Ok(())
		}
		Err(error) => Err(error),
	}
}

fn process_connection_update(
	conn_update_result: Result<ConnectionUpdate, RecvError>,
	user: &mut Option<UserData>,
	event_permission_cache: &mut HashMap<Event, Option<Permission>>,
) -> Result<Option<Box<dyn Serialize + Send + Sync>>, HandleConnectionError> {
	match conn_update_result {
		Ok(conn_update) => match conn_update {
			ConnectionUpdate::SendData(send_message) => Ok(Some(send_message)),
			ConnectionUpdate::UserUpdate(user_data_update) => {
				match user_data_update {
					UserDataUpdate::User(new_user_data) => *user = Some(new_user_data),
					UserDataUpdate::EventPermissions(event, new_permission) => {
						event_permission_cache.insert(event, new_permission);
					}
				}
				if let Some(user) = user.clone() {
					let available_events: Vec<Event> = event_permission_cache
						.iter()
						.filter(|(_, permission)| permission.is_some())
						.map(|(event, _)| event.clone())
						.collect();
					let user_subscription_data = UserSubscriptionUpdate { user, available_events };
					let message = FromServerMessage::SubscriptionMessage(Box::new(SubscriptionData::UserUpdate(
						user_subscription_data,
					)));
					Ok(Some(Box::new(message)))
				} else {
					Ok(None)
				}
			}
		},
		Err(_) => Err(HandleConnectionError::ConnectionClosed),
	}
}

struct ProcessIncomingMessageParams<'a> {
	recv_msg_result: Result<String, WebSocketRecvError>,
	db_connection: &'a Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &'a mut Option<UserData>,
	connection_id: &'a str,
	subscription_manager: &'a Arc<Mutex<SubscriptionManager>>,
	openid_user_id: &'a str,
	event_permission_cache: &'a mut HashMap<Event, Option<Permission>>,
}

async fn process_incoming_message(args: ProcessIncomingMessageParams<'_>) -> Result<(), HandleConnectionError> {
	let incoming_msg = {
		match args.recv_msg_result {
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
			let Some(user) = args.user.as_ref() else {
				return Ok(());
			}; // Only logged-in users can subscribe
			match subscription_type {
				SubscriptionType::EventLogData(event_id) => {
					subscribe_to_event(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						&event_id,
						args.event_permission_cache,
					)
					.await?
				}
				SubscriptionType::AdminUsers => {
					subscribe_to_admin_users(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
					)
					.await?
				}
				SubscriptionType::AdminEvents => {
					subscribe_to_admin_events(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
					)
					.await?
				}
				SubscriptionType::AdminPermissionGroups => {
					subscribe_to_admin_permission_groups(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
					)
					.await?
				}
				SubscriptionType::AdminPermissionGroupUsers => {
					subscribe_to_admin_permission_groups_users(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
					)
					.await?
				}
				SubscriptionType::AdminEntryTypes => {
					subscribe_to_admin_entry_types(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
					)
					.await?
				}
				SubscriptionType::AdminEntryTypesEvents => {
					subscribe_to_admin_entry_types_events(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
					)
					.await?
				}
				SubscriptionType::AdminEventEditors => {
					subscribe_to_admin_editors(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
					)
					.await?
				}
				SubscriptionType::AdminEventLogTabs => {
					subscribe_to_admin_event_log_tabs(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
					)
					.await?
				}
				SubscriptionType::AdminApplications => {
					subscribe_to_admin_applications(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
					)
					.await?
				}
				SubscriptionType::AdminInfoPages => {
					subscribe_to_admin_info_pages(
						Arc::clone(args.db_connection),
						args.conn_update_tx,
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
					)
					.await?
				}
			}
		}
		FromClientMessage::EndSubscription(subscription_type) => {
			let subscription_manager = args.subscription_manager.lock().await;
			match subscription_type {
				SubscriptionType::EventLogData(event_id) => {
					subscription_manager
						.unsubscribe_from_event(&event_id, args.connection_id)
						.await?
				}
				SubscriptionType::AdminUsers => {
					subscription_manager
						.remove_admin_user_subscription(args.connection_id)
						.await?
				}
				SubscriptionType::AdminEvents => {
					subscription_manager
						.remove_admin_event_subscription(args.connection_id)
						.await?
				}
				SubscriptionType::AdminPermissionGroups => {
					subscription_manager
						.remove_admin_permission_group_subscription(args.connection_id)
						.await?
				}
				SubscriptionType::AdminPermissionGroupUsers => {
					subscription_manager
						.remove_admin_permission_group_users_subscription(args.connection_id)
						.await?
				}
				SubscriptionType::AdminEntryTypes => {
					subscription_manager
						.remove_admin_entry_types_subscription(args.connection_id)
						.await?
				}
				SubscriptionType::AdminEntryTypesEvents => {
					subscription_manager
						.remove_admin_entry_types_events_subscription(args.connection_id)
						.await?
				}
				SubscriptionType::AdminEventEditors => {
					subscription_manager
						.remove_admin_editors_subscription(args.connection_id)
						.await?
				}
				SubscriptionType::AdminEventLogTabs => {
					subscription_manager
						.remove_admin_event_log_tabs_subscription(args.connection_id)
						.await?
				}
				SubscriptionType::AdminApplications => {
					subscription_manager
						.remove_admin_applications_subscription(args.connection_id)
						.await?
				}
				SubscriptionType::AdminInfoPages => {
					subscription_manager
						.remove_admin_info_pages_subscription(args.connection_id)
						.await?
				}
			}
		}
		FromClientMessage::SubscriptionMessage(subscription_update) => {
			let Some(user) = args.user.as_ref() else {
				return Ok(());
			}; // One must be subscribed (and therefore logged in) to send a subscription update message
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
				SubscriptionTargetUpdate::AdminEventsUpdate(update_data) => {
					handle_admin_event_message(
						Arc::clone(args.db_connection),
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						update_data,
					)
					.await
				}
				SubscriptionTargetUpdate::AdminEntryTypesUpdate(update_data) => {
					handle_admin_entry_type_message(
						Arc::clone(args.db_connection),
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						update_data,
					)
					.await
				}
				SubscriptionTargetUpdate::AdminEntryTypesEventsUpdate(update_data) => {
					handle_admin_entry_type_event_message(
						Arc::clone(args.db_connection),
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						update_data,
					)
					.await
				}
				SubscriptionTargetUpdate::AdminPermissionGroupsUpdate(update_data) => {
					handle_admin_permission_groups_message(
						Arc::clone(args.db_connection),
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						update_data,
					)
					.await
				}
				SubscriptionTargetUpdate::AdminUserUpdate(modified_user) => {
					handle_admin_users_message(
						Arc::clone(args.db_connection),
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						&modified_user,
					)
					.await
				}
				SubscriptionTargetUpdate::AdminEventEditorsUpdate(update_data) => {
					handle_admin_editors_message(
						Arc::clone(args.db_connection),
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						update_data,
					)
					.await
				}
				SubscriptionTargetUpdate::AdminUserPermissionGroupsUpdate(update_data) => {
					handle_admin_permission_group_users_message(
						Arc::clone(args.db_connection),
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						update_data,
					)
					.await
				}
				SubscriptionTargetUpdate::AdminEventLogTabsUpdate(update_data) => {
					handle_admin_event_log_tabs_message(
						Arc::clone(args.db_connection),
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						update_data,
					)
					.await
				}
				SubscriptionTargetUpdate::AdminApplicationsUpdate(update_data) => {
					handle_admin_applications_message(
						Arc::clone(args.db_connection),
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						update_data,
						args.conn_update_tx,
					)
					.await
				}
				SubscriptionTargetUpdate::AdminInfoPagesUpdate(update_data) => {
					handle_admin_info_pages_message(
						Arc::clone(args.db_connection),
						args.connection_id,
						user,
						Arc::clone(args.subscription_manager),
						update_data,
					)
					.await
				}
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
							args.connection_id,
							args.openid_user_id,
							registration_data,
							args.user,
							Arc::clone(args.subscription_manager),
						)
						.await?
					}
				}
			}
		}
		FromClientMessage::UpdateProfile(profile_data) => {
			if let Some(user) = args.user.as_ref() {
				handle_profile_update(
					Arc::clone(args.db_connection),
					user,
					Arc::clone(args.subscription_manager),
					profile_data,
				)
				.await?;
			}
		}
	};

	Ok(())
}
