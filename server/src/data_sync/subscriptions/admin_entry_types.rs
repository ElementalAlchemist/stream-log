// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::{AvailableEntryType, EntryType as EntryTypeDb, Event as EventDb};
use crate::schema::{available_entry_types_for_event, entry_types, events};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{
	AdminEntryTypeData, AdminEntryTypeEventData, AdminEntryTypeEventUpdate, AdminEntryTypeUpdate,
	EntryTypeEventAssociation,
};
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::SelfUserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_entry_types(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &SelfUserData,
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
		Ok(types) => types.into_iter().map(|entry_type| entry_type.into()).collect(),
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

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_entry_types_subscription(connection_id, conn_update_tx.clone())
		.await;

	let message =
		FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminEntryTypes(entry_types)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_admin_entry_type_message(
	db_connection: Arc<Mutex<PgConnection>>,
	connection_id: &str,
	user: &SelfUserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminEntryTypeUpdate,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.is_subscribed_to_admin_entry_types(connection_id)
		.await
	{
		return;
	}

	match update_message {
		AdminEntryTypeUpdate::UpdateEntryType(mut entry_type) => {
			let (update_result, event_data_result) = {
				let mut db_connection = db_connection.lock().await;
				let update_result = if entry_type.id.is_empty() {
					entry_type.id = cuid2::create_id();
					let db_entry_type = EntryTypeDb {
						id: entry_type.id.clone(),
						name: entry_type.name.clone(),
						description: entry_type.description.clone(),
						color_red: entry_type.color.r.into(),
						color_green: entry_type.color.g.into(),
						color_blue: entry_type.color.b.into(),
						require_end_time: entry_type.require_end_time,
					};
					diesel::insert_into(entry_types::table)
						.values(db_entry_type)
						.execute(&mut *db_connection)
				} else {
					let red: i32 = entry_type.color.r.into();
					let green: i32 = entry_type.color.g.into();
					let blue: i32 = entry_type.color.b.into();
					diesel::update(entry_types::table)
						.filter(entry_types::id.eq(&entry_type.id))
						.set((
							entry_types::name.eq(&entry_type.name),
							entry_types::description.eq(&entry_type.description),
							entry_types::color_red.eq(red),
							entry_types::color_green.eq(green),
							entry_types::color_blue.eq(blue),
							entry_types::require_end_time.eq(entry_type.require_end_time),
						))
						.execute(&mut *db_connection)
				};

				let event_ids: QueryResult<Vec<String>> = available_entry_types_for_event::table
					.filter(available_entry_types_for_event::entry_type.eq(&entry_type.id))
					.select(available_entry_types_for_event::event_id)
					.load(&mut *db_connection);
				match event_ids {
					Ok(event_ids) => {
						let events: QueryResult<Vec<EventDb>> = events::table
							.filter(events::id.eq_any(&event_ids))
							.load(&mut *db_connection);
						(update_result, events)
					}
					Err(error) => (update_result, Err(error)),
				}
			};
			if let Err(error) = update_result {
				tide::log::error!("A database error occurred updating an entry type: {}", error);
				return;
			}
			let events = match event_data_result {
				Ok(events) => events,
				Err(error) => {
					tide::log::error!("A database error occurred getting events for an entry type: {}", error);
					return;
				}
			};

			let subscription_manager = subscription_manager.lock().await;
			let admin_message =
				SubscriptionData::AdminEntryTypesUpdate(AdminEntryTypeData::UpdateEntryType(entry_type.clone()));
			let send_result = subscription_manager
				.broadcast_admin_entry_types_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to broadcast admin entry type update message: {}", error);
			}

			for event in events {
				let event: Event = event.into();
				let event_id = event.id.clone();
				let event_message = SubscriptionData::EventUpdate(
					event,
					Box::new(EventSubscriptionData::UpdateEntryType(entry_type.clone())),
				);
				let send_result = subscription_manager
					.broadcast_event_message(&event_id, event_message)
					.await;
				if let Err(error) = send_result {
					tide::log::error!("Failed to broadcast event entry type update message: {}", error);
				}
			}
		}
	}
}

pub async fn subscribe_to_admin_entry_types_events(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &SelfUserData,
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
		Ok(events) => {
			let events_map: HashMap<String, Event> = events
				.into_iter()
				.map(|event| (event.id.clone(), event.into()))
				.collect();
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
		Ok(entry_types) => {
			let entry_types_map: HashMap<String, EntryType> = entry_types
				.into_iter()
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

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_entry_types_events_subscription(connection_id, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(
		InitialSubscriptionLoadData::AdminEntryTypesEvents(entry_type_event_associations),
	));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_admin_entry_type_event_message(
	db_connection: Arc<Mutex<PgConnection>>,
	connection_id: &str,
	user: &SelfUserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminEntryTypeEventUpdate,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.is_subscribed_to_admin_entry_types_events(connection_id)
		.await
	{
		return;
	}

	let (admin_message, event_id, event_message) = match update_message {
		AdminEntryTypeEventUpdate::AddTypeToEvent(association) => {
			let mut db_connection = db_connection.lock().await;
			let available_entry_type = AvailableEntryType {
				entry_type: association.entry_type.id.clone(),
				event_id: association.event.id.clone(),
			};
			let insert_result = diesel::insert_into(available_entry_types_for_event::table)
				.values(available_entry_type)
				.execute(&mut *db_connection);
			if let Err(error) = insert_result {
				tide::log::error!(
					"A database error occurred adding event type + entry association: {}",
					error
				);
				return;
			}

			let event_id = association.event.id.clone();
			let admin_message = SubscriptionData::AdminEntryTypesEventsUpdate(AdminEntryTypeEventData::AddTypeToEvent(
				association.clone(),
			));
			let event_message = SubscriptionData::EventUpdate(
				association.event,
				Box::new(EventSubscriptionData::AddEntryType(association.entry_type)),
			);
			(admin_message, event_id, event_message)
		}
		AdminEntryTypeEventUpdate::RemoveTypeFromEvent(association) => {
			let mut db_connection = db_connection.lock().await;
			let delete_result = diesel::delete(available_entry_types_for_event::table)
				.filter(
					available_entry_types_for_event::entry_type
						.eq(&association.entry_type.id)
						.and(available_entry_types_for_event::event_id.eq(&association.event.id)),
				)
				.execute(&mut *db_connection);
			if let Err(error) = delete_result {
				tide::log::error!(
					"A database error occurred deleting event type + entry association: {}",
					error
				);
				return;
			}

			let event_id = association.event.id.clone();
			let admin_message = SubscriptionData::AdminEntryTypesEventsUpdate(
				AdminEntryTypeEventData::RemoveTypeFromEvent(association.clone()),
			);
			let event_message = SubscriptionData::EventUpdate(
				association.event,
				Box::new(EventSubscriptionData::DeleteEntryType(association.entry_type)),
			);
			(admin_message, event_id, event_message)
		}
	};

	let subscription_manager = subscription_manager.lock().await;
	let send_result = subscription_manager
		.broadcast_admin_entry_types_events_message(admin_message)
		.await;
	if let Err(error) = send_result {
		tide::log::error!(
			"Failed to broadcast entry type and event update to administrators: {}",
			error
		);
	}

	let send_result = subscription_manager
		.broadcast_event_message(&event_id, event_message)
		.await;
	if let Err(error) = send_result {
		tide::log::error!("Failed to broadcast entry type and event update to users: {}", error);
	}
}
