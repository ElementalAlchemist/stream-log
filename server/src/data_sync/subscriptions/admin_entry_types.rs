use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::{AvailableEntryType, EntryType as EntryTypeDb, Event as EventDb};
use crate::schema::{available_entry_types_for_event, entry_types, events};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::admin::EntryTypeEventAssociation;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_entry_types(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message = FromServerMessage::SubscriptionFailure(
			SubscriptionType::AdminEntryTypes,
			SubscriptionFailureInfo::NotAllowed,
		);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = db_connection.lock().await;
	let entry_types: QueryResult<Vec<EntryTypeDb>> = entry_types::table.load(&mut *db_connection);

	let entry_types: Vec<EntryType> = match entry_types {
		Ok(mut types) => types.drain(..).map(|entry_type| entry_type.into()).collect(),
		Err(error) => {
			tide::log::error!(
				"A database error occurred getting admin entry type subscription data: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEntryTypes,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let mut subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_entry_types_subscription(user, conn_update_tx.clone())
		.await;

	let message =
		FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminEntryTypes(entry_types)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn subscribe_to_admin_entry_types_events(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message = FromServerMessage::SubscriptionFailure(
			SubscriptionType::AdminEntryTypesEvents,
			SubscriptionFailureInfo::NotAllowed,
		);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = db_connection.lock().await;
	let entry_type_events: QueryResult<Vec<AvailableEntryType>> =
		available_entry_types_for_event::table.load(&mut *db_connection);
	let entry_type_events = match entry_type_events {
		Ok(data) => data,
		Err(error) => {
			tide::log::error!(
				"A database error occurred retrieving entry type and event associations for a subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEntryTypesEvents,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let event_ids: Vec<String> = entry_type_events.iter().map(|data| data.event_id.clone()).collect();
	let entry_type_ids: Vec<String> = entry_type_events.iter().map(|data| data.entry_type.clone()).collect();

	let events: QueryResult<Vec<EventDb>> = events::table
		.filter(events::id.eq_any(&event_ids))
		.load(&mut *db_connection);
	let entry_types: QueryResult<Vec<EntryTypeDb>> = entry_types::table
		.filter(entry_types::id.eq_any(&entry_type_ids))
		.load(&mut *db_connection);

	let events = match events {
		Ok(mut events) => {
			let events_map: HashMap<String, Event> =
				events.drain(..).map(|event| (event.id.clone(), event.into())).collect();
			events_map
		}
		Err(error) => {
			tide::log::error!(
				"A database error occurred retrieving event data for entry type associations: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEntryTypesEvents,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};
	let entry_types = match entry_types {
		Ok(mut entry_types) => {
			let entry_types_map: HashMap<String, EntryType> = entry_types
				.drain(..)
				.map(|entry_type| (entry_type.id.clone(), entry_type.into()))
				.collect();
			entry_types_map
		}
		Err(error) => {
			tide::log::error!(
				"A database error occurred retrieving entry type data for event associations: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEntryTypesEvents,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let mut entry_type_event_associations: Vec<EntryTypeEventAssociation> = Vec::with_capacity(entry_type_events.len());
	for entry_type_event in entry_type_events.iter() {
		let entry_type = entry_types.get(&entry_type_event.entry_type).unwrap().clone();
		let event = events.get(&entry_type_event.event_id).unwrap().clone();
		entry_type_event_associations.push(EntryTypeEventAssociation { entry_type, event });
	}

	let mut subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_entry_types_events_subscription(user, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(
		InitialSubscriptionLoadData::AdminEntryTypesEvents(entry_type_event_associations),
	));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}
