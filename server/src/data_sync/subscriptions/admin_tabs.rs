// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::send_lost_db_connection_subscription_response;
use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::{Event as EventDb, EventLogTab as EventLogTabDb};
use crate::schema::{event_log_tabs, events};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use std::collections::HashMap;
use stream_log_shared::messages::admin::{AdminEventLogTabsData, AdminEventLogTabsUpdate};
use stream_log_shared::messages::event_log::EventLogTab;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::SelfUserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_event_log_tabs(
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &SelfUserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message = FromServerMessage::SubscriptionFailure(
			SubscriptionType::AdminEventLogTabs,
			SubscriptionFailureInfo::NotAllowed,
		);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = match db_connection_pool.get() {
		Ok(connection) => connection,
		Err(error) => {
			send_lost_db_connection_subscription_response(error, &conn_update_tx, SubscriptionType::AdminEventLogTabs)
				.await?;
			return Ok(());
		}
	};
	let db_data: QueryResult<(Vec<EventLogTabDb>, Vec<EventDb>)> = db_connection.transaction(|db_connection| {
		let tabs = event_log_tabs::table.load(db_connection)?;
		let events = events::table.load(db_connection)?;
		Ok((tabs, events))
	});
	let (tabs, events) = match db_data {
		Ok(data) => data,
		Err(error) => {
			tide::log::error!(
				"A database error occurred getting event log tabs for an admin subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEventLogTabs,
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

	let tabs: Vec<(Event, EventLogTab)> = tabs
		.iter()
		.map(|tab| {
			(
				events_by_id.get(&tab.event).cloned().unwrap(),
				EventLogTab {
					id: tab.id.clone(),
					name: tab.name.clone(),
					start_time: tab.start_time,
				},
			)
		})
		.collect();

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_event_log_tabs_subscription(connection_id, conn_update_tx.clone())
		.await;

	let message =
		FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminEventLogTabs(tabs)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;
	Ok(())
}

pub async fn handle_admin_event_log_tabs_message(
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	connection_id: &str,
	user: &SelfUserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminEventLogTabsUpdate,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.is_subscribed_to_admin_event_log_tabs(connection_id)
		.await
	{
		return;
	}

	match update_message {
		AdminEventLogTabsUpdate::AddTab(event, mut tab) => {
			let tab_id = cuid2::create_id();
			let new_tab = EventLogTabDb {
				id: tab_id.clone(),
				event: event.id.clone(),
				name: tab.name.clone(),
				start_time: tab.start_time,
			};
			let db_result: QueryResult<_> = {
				let mut db_connection = match db_connection_pool.get() {
					Ok(connection) => connection,
					Err(error) => {
						tide::log::error!(
							"A database connection error occurred adding an event log tab: {}",
							error
						);
						return;
					}
				};
				diesel::insert_into(event_log_tabs::table)
					.values(new_tab)
					.execute(&mut *db_connection)
			};
			if let Err(error) = db_result {
				tide::log::error!("A database error occurred adding an event log tab: {}", error);
				return;
			}

			tab.id = tab_id;
			let subscription_manager = subscription_manager.lock().await;
			let event_message =
				SubscriptionData::EventUpdate(event.clone(), Box::new(EventSubscriptionData::UpdateTab(tab.clone())));
			let send_result = subscription_manager
				.broadcast_event_message(&event.id, event_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send event update for new event log tab: {}", error);
			}
			let admin_message = SubscriptionData::AdminEventLogTabsUpdate(AdminEventLogTabsData::AddTab(event, tab));
			let send_result = subscription_manager
				.broadcast_admin_event_log_tabs_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send admin update for new event log tab: {}", error);
			}
		}
		AdminEventLogTabsUpdate::UpdateTab(tab) => {
			let db_result: QueryResult<EventDb> = {
				let mut db_connection = match db_connection_pool.get() {
					Ok(connection) => connection,
					Err(error) => {
						tide::log::error!(
							"A database connection error occurred updating an event log tab: {}",
							error
						);
						return;
					}
				};
				db_connection.transaction(|db_connection| {
					let db_tab: EventLogTabDb = diesel::update(event_log_tabs::table)
						.filter(event_log_tabs::id.eq(&tab.id))
						.set((
							event_log_tabs::name.eq(&tab.name),
							event_log_tabs::start_time.eq(tab.start_time),
						))
						.get_result(db_connection)?;
					let event: EventDb = events::table.find(&db_tab.event).first(db_connection)?;
					Ok(event)
				})
			};
			let event: Event = match db_result {
				Ok(event) => event.into(),
				Err(error) => {
					tide::log::error!("A database error occurred updating an event log tab: {}", error);
					return;
				}
			};

			let subscription_manager = subscription_manager.lock().await;
			let event_id = event.id.clone();
			let event_message =
				SubscriptionData::EventUpdate(event, Box::new(EventSubscriptionData::UpdateTab(tab.clone())));
			let send_result = subscription_manager
				.broadcast_event_message(&event_id, event_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send event update for event log section: {}", error);
			}
			let admin_message = SubscriptionData::AdminEventLogTabsUpdate(AdminEventLogTabsData::UpdateTab(tab));
			let send_result = subscription_manager
				.broadcast_admin_event_log_tabs_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send admin update for event log tab: {}", error);
			}
		}
		AdminEventLogTabsUpdate::DeleteTab(tab) => {
			let db_result: QueryResult<EventDb> = {
				let mut db_connection = match db_connection_pool.get() {
					Ok(connection) => connection,
					Err(error) => {
						tide::log::error!(
							"A database connection error occurred deleting an event log tab: {}",
							error
						);
						return;
					}
				};
				db_connection.transaction(|db_connection| {
					let db_section: EventLogTabDb = diesel::delete(event_log_tabs::table)
						.filter(event_log_tabs::id.eq(&tab.id))
						.get_result(db_connection)?;
					let event: EventDb = events::table.find(&db_section.event).first(db_connection)?;
					Ok(event)
				})
			};
			let event: Event = match db_result {
				Ok(event) => event.into(),
				Err(error) => {
					tide::log::error!("A database error occurred deleting an event log tab: {}", error);
					return;
				}
			};

			let subscription_manager = subscription_manager.lock().await;
			let event_id = event.id.clone();
			let event_message =
				SubscriptionData::EventUpdate(event, Box::new(EventSubscriptionData::DeleteTab(tab.clone())));
			let send_result = subscription_manager
				.broadcast_event_message(&event_id, event_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send event update for deleting event log tab: {}", error);
			}
			let admin_message = SubscriptionData::AdminEventLogTabsUpdate(AdminEventLogTabsData::DeleteTab(tab));
			let send_result = subscription_manager
				.broadcast_admin_event_log_tabs_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send admin update for deleting event log tab: {}", error);
			}
		}
	}
}
