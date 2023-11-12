use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::Event as EventDb;
use crate::schema::events;
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use stream_log_shared::messages::admin::{AdminEventData, AdminEventUpdate};
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_events(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message =
			FromServerMessage::SubscriptionFailure(SubscriptionType::AdminEvents, SubscriptionFailureInfo::NotAllowed);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = db_connection.lock().await;
	let events: QueryResult<Vec<EventDb>> = events::table.load(&mut *db_connection);
	let events: Vec<Event> = match events {
		Ok(events) => events.into_iter().map(|event| event.into()).collect(),
		Err(error) => {
			tide::log::error!("A database error occurred getting the admin events list: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEvents,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_event_subscription(connection_id, conn_update_tx.clone())
		.await;

	let message =
		FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminEvents(events)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_admin_event_message(
	db_connection: Arc<Mutex<PgConnection>>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminEventUpdate,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.is_subscribed_to_admin_events(connection_id)
		.await
	{
		return;
	}

	match update_message {
		AdminEventUpdate::UpdateEvent(mut event) => {
			let db_result = {
				let mut db_connection = db_connection.lock().await;
				if event.id.is_empty() {
					event.id = cuid2::create_id();
					let event_db = EventDb {
						id: event.id.clone(),
						name: event.name.clone(),
						start_time: event.start_time,
						editor_link_format: event.editor_link_format.clone(),
						default_first_tab_name: event.default_first_tab_name.clone(),
					};
					diesel::insert_into(events::table)
						.values(event_db)
						.execute(&mut *db_connection)
				} else {
					diesel::update(events::table)
						.filter(events::id.eq(&event.id))
						.set((
							events::name.eq(&event.name),
							events::start_time.eq(event.start_time),
							events::editor_link_format.eq(&event.editor_link_format),
							events::default_first_tab_name.eq(&event.default_first_tab_name),
						))
						.execute(&mut *db_connection)
				}
			};
			if let Err(error) = db_result {
				tide::log::error!("A database error occurred updating event data: {}", error);
				return;
			}

			let subscription_manager = subscription_manager.lock().await;
			let admin_message = SubscriptionData::AdminEventsUpdate(AdminEventData::UpdateEvent(event.clone()));
			let broadcast_result = subscription_manager.broadcast_admin_event_message(admin_message).await;
			if let Err(error) = broadcast_result {
				tide::log::error!("Failed to broadcast an admin event update: {}", error);
			}

			let event_id = event.id.clone();
			let event_message =
				SubscriptionData::EventUpdate(event.clone(), Box::new(EventSubscriptionData::UpdateEvent));
			let broadcast_result = subscription_manager
				.broadcast_event_message(&event_id, event_message)
				.await;
			if let Err(error) = broadcast_result {
				tide::log::error!("Failed to broadcast an event update: {}", error);
			}
		}
	}
}
