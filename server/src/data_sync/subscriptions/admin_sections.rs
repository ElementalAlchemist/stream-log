use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::{Event as EventDb, EventLogSection as EventLogSectionDb};
use crate::schema::{event_log_sections, events};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{AdminEventLogSectionsData, AdminEventLogSectionsUpdate};
use stream_log_shared::messages::event_log::EventLogSection;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_event_log_sections(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message = FromServerMessage::SubscriptionFailure(
			SubscriptionType::AdminEventLogSections,
			SubscriptionFailureInfo::NotAllowed,
		);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = db_connection.lock().await;
	let db_data: QueryResult<(Vec<EventLogSectionDb>, Vec<EventDb>)> = db_connection.transaction(|db_connection| {
		let sections = event_log_sections::table.load(db_connection)?;
		let events = events::table.load(db_connection)?;
		Ok((sections, events))
	});
	let (sections, events) = match db_data {
		Ok(data) => data,
		Err(error) => {
			tide::log::error!(
				"A database error occurred getting event log sections for an admin subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEventLogSections,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let events_by_id: HashMap<String, Event> = events
		.iter()
		.map(|event| (event.id.clone(), event.clone().into()))
		.collect();

	let sections: Vec<(Event, EventLogSection)> = sections
		.iter()
		.map(|section| {
			(
				events_by_id.get(&section.event).cloned().unwrap(),
				EventLogSection {
					id: section.id.clone(),
					name: section.name.clone(),
					start_time: section.start_time,
				},
			)
		})
		.collect();

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_event_log_sections_subscription(connection_id, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(
		InitialSubscriptionLoadData::AdminEventLogSections(sections),
	));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;
	Ok(())
}

pub async fn handle_admin_event_log_sections_message(
	db_connection: Arc<Mutex<PgConnection>>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminEventLogSectionsUpdate,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.is_subscribed_to_admin_event_log_sections(connection_id)
		.await
	{
		return;
	}

	match update_message {
		AdminEventLogSectionsUpdate::AddSection(event, mut section) => {
			let section_id = cuid2::create_id();
			let new_section = EventLogSectionDb {
				id: section_id.clone(),
				event: event.id.clone(),
				name: section.name.clone(),
				start_time: section.start_time,
			};
			let db_result: QueryResult<_> = {
				let mut db_connection = db_connection.lock().await;
				diesel::insert_into(event_log_sections::table)
					.values(new_section)
					.execute(&mut *db_connection)
			};
			if let Err(error) = db_result {
				tide::log::error!("A database error occurred adding an event log section: {}", error);
				return;
			}

			section.id = section_id;
			let subscription_manager = subscription_manager.lock().await;
			let event_message = SubscriptionData::EventUpdate(
				event.clone(),
				Box::new(EventSubscriptionData::UpdateSection(section.clone())),
			);
			let send_result = subscription_manager
				.broadcast_event_message(&event.id, event_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send event update for new event log section: {}", error);
			}
			let admin_message =
				SubscriptionData::AdminEventLogSectionsUpdate(AdminEventLogSectionsData::AddSection(event, section));
			let send_result = subscription_manager
				.broadcast_admin_event_log_sections_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send admin update for new event log section: {}", error);
			}
		}
		AdminEventLogSectionsUpdate::UpdateSection(section) => {
			let db_result: QueryResult<EventDb> = {
				let mut db_connection = db_connection.lock().await;
				db_connection.transaction(|db_connection| {
					let db_section: EventLogSectionDb = diesel::update(event_log_sections::table)
						.filter(event_log_sections::id.eq(&section.id))
						.set((
							event_log_sections::name.eq(&section.name),
							event_log_sections::start_time.eq(section.start_time),
						))
						.get_result(db_connection)?;
					let event: EventDb = events::table.find(&db_section.event).first(db_connection)?;
					Ok(event)
				})
			};
			let event: Event = match db_result {
				Ok(event) => event.into(),
				Err(error) => {
					tide::log::error!("A database error occurred updating an event log section: {}", error);
					return;
				}
			};

			let subscription_manager = subscription_manager.lock().await;
			let event_id = event.id.clone();
			let event_message =
				SubscriptionData::EventUpdate(event, Box::new(EventSubscriptionData::UpdateSection(section.clone())));
			let send_result = subscription_manager
				.broadcast_event_message(&event_id, event_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send event update for event log section: {}", error);
			}
			let admin_message =
				SubscriptionData::AdminEventLogSectionsUpdate(AdminEventLogSectionsData::UpdateSection(section));
			let send_result = subscription_manager
				.broadcast_admin_event_log_sections_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send admin update for event log section: {}", error);
			}
		}
		AdminEventLogSectionsUpdate::DeleteSection(section) => {
			let db_result: QueryResult<EventDb> = {
				let mut db_connection = db_connection.lock().await;
				db_connection.transaction(|db_connection| {
					let db_section: EventLogSectionDb = diesel::delete(event_log_sections::table)
						.filter(event_log_sections::id.eq(&section.id))
						.get_result(db_connection)?;
					let event: EventDb = events::table.find(&db_section.event).first(db_connection)?;
					Ok(event)
				})
			};
			let event: Event = match db_result {
				Ok(event) => event.into(),
				Err(error) => {
					tide::log::error!("A database error occurred deleting an event log section: {}", error);
					return;
				}
			};

			let subscription_manager = subscription_manager.lock().await;
			let event_id = event.id.clone();
			let event_message =
				SubscriptionData::EventUpdate(event, Box::new(EventSubscriptionData::DeleteSection(section.clone())));
			let send_result = subscription_manager
				.broadcast_event_message(&event_id, event_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send event update for deleting event log section: {}", error);
			}
			let admin_message =
				SubscriptionData::AdminEventLogSectionsUpdate(AdminEventLogSectionsData::DeleteSection(section));
			let send_result = subscription_manager
				.broadcast_admin_event_log_sections_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send admin update for deleting event log section: {}", error);
			}
		}
	}
}
