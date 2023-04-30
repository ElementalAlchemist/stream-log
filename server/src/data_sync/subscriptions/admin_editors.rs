use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::{Event as EventDb, EventEditor, User};
use crate::schema::{event_editors, events, users};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{AdminEventEditorData, AdminEventEditorUpdate, EditorEventAssociation};
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_editors(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message = FromServerMessage::SubscriptionFailure(
			SubscriptionType::AdminEventEditors,
			SubscriptionFailureInfo::NotAllowed,
		);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = db_connection.lock().await;
	let event_editors: QueryResult<Vec<EventEditor>> = event_editors::table.load(&mut *db_connection);
	let event_editor_ids = match event_editors {
		Ok(ids) => ids,
		Err(error) => {
			tide::log::error!(
				"A database error occurred loading event editors for admin subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEventEditors,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};
	let event_ids: Vec<String> = event_editor_ids
		.iter()
		.map(|event_editor| event_editor.event.clone())
		.collect();
	let editor_ids: Vec<String> = event_editor_ids
		.iter()
		.map(|event_editor| event_editor.editor.clone())
		.collect();

	let users: QueryResult<Vec<User>> = users::table
		.filter(users::id.eq_any(&editor_ids))
		.load(&mut *db_connection);
	let events: QueryResult<Vec<EventDb>> = events::table
		.filter(events::id.eq_any(&event_ids))
		.load(&mut *db_connection);

	let users = match users {
		Ok(mut users) => {
			let user_map: HashMap<String, UserData> =
				users.drain(..).map(|user| (user.id.clone(), user.into())).collect();
			user_map
		}
		Err(error) => {
			tide::log::error!(
				"A database error occurred getting users for admin editor subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEventEditors,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};
	let events = match events {
		Ok(mut events) => {
			let event_map: HashMap<String, Event> =
				events.drain(..).map(|event| (event.id.clone(), event.into())).collect();
			event_map
		}
		Err(error) => {
			tide::log::error!(
				"A database error occurred getting events for admin editor subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEventEditors,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let mut event_editors: Vec<EditorEventAssociation> = Vec::with_capacity(event_editor_ids.len());
	for event_editor in event_editor_ids.iter() {
		let editor = users.get(&event_editor.editor).unwrap().clone();
		let event = events.get(&event_editor.event).unwrap().clone();
		event_editors.push(EditorEventAssociation { editor, event });
	}

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_editors_subscription(user, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminEventEditors(
		event_editors,
	)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_admin_editors_message(
	db_connection: Arc<Mutex<PgConnection>>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminEventEditorUpdate,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.user_is_subscribed_to_admin_editors(user)
		.await
	{
		return;
	}

	match update_message {
		AdminEventEditorUpdate::AddEditor(editor_data) => {
			{
				let mut db_connection = db_connection.lock().await;
				let event_editor = EventEditor {
					event: editor_data.event.id.clone(),
					editor: editor_data.editor.id.clone(),
				};
				let db_result = diesel::insert_into(event_editors::table)
					.values(event_editor)
					.execute(&mut *db_connection);
				if let Err(error) = db_result {
					tide::log::error!("A database error occurred adding an editor for an event: {}", error);
					return;
				}
			}

			let subscription_manager = subscription_manager.lock().await;
			let event_message = SubscriptionData::EventUpdate(
				editor_data.event.clone(),
				Box::new(EventSubscriptionData::AddEditor(editor_data.editor.clone())),
			);
			let send_result = subscription_manager
				.broadcast_event_message(&editor_data.event.id, event_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send editor update to event subscription: {}", error);
			}

			let admin_message = SubscriptionData::AdminEventEditorsUpdate(AdminEventEditorData::AddEditor(editor_data));
			let send_result = subscription_manager
				.broadcast_admin_editors_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send editor update to admin subscription: {}", error);
			}
		}
		AdminEventEditorUpdate::RemoveEditor(editor_data) => {
			{
				let mut db_connection = db_connection.lock().await;
				let db_result = diesel::delete(event_editors::table)
					.filter(
						event_editors::event
							.eq(&editor_data.event.id)
							.and(event_editors::editor.eq(&editor_data.editor.id)),
					)
					.execute(&mut *db_connection);
				if let Err(error) = db_result {
					tide::log::error!("A database error occurred removing an editor from an event: {}", error);
					return;
				}
			}

			let subscription_manager = subscription_manager.lock().await;
			let event_message = SubscriptionData::EventUpdate(
				editor_data.event.clone(),
				Box::new(EventSubscriptionData::RemoveEditor(editor_data.editor.clone())),
			);
			let send_result = subscription_manager
				.broadcast_event_message(&editor_data.event.id, event_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send editor removal to event subscription: {}", error);
			}

			let admin_message =
				SubscriptionData::AdminEventEditorsUpdate(AdminEventEditorData::RemoveEditor(editor_data));
			let send_result = subscription_manager
				.broadcast_admin_editors_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send editor removal to admin subscription: {}", error);
			}
		}
	}
}
