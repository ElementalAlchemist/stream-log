// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::{Event as EventDb, InfoPage as InfoPageDb};
use crate::schema::{events, info_pages};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{AdminInfoPageData, AdminInfoPageUpdate};
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::info_pages::InfoPage;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::SelfUserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_info_pages(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &SelfUserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message = FromServerMessage::SubscriptionFailure(
			SubscriptionType::AdminInfoPages,
			SubscriptionFailureInfo::NotAllowed,
		);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let query_result: QueryResult<(Vec<EventDb>, Vec<InfoPageDb>)> = {
		let mut db_connection = db_connection.lock().await;
		db_connection.transaction(|db_connection| {
			let info_pages: Vec<InfoPageDb> = info_pages::table.load(db_connection)?;
			let events: Vec<EventDb> = events::table.load(db_connection)?;
			Ok((events, info_pages))
		})
	};

	let (events, info_pages) = match query_result {
		Ok(data) => data,
		Err(error) => {
			tide::log::error!(
				"A database error occurred retrieving info pages for admin subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminInfoPages,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let events: HashMap<String, Event> = events
		.into_iter()
		.map(|event| (event.id.clone(), event.into()))
		.collect();

	let info_pages: Vec<InfoPage> = info_pages
		.into_iter()
		.map(|page| InfoPage {
			id: page.id,
			event: events.get(&page.event).unwrap().clone(),
			title: page.title,
			contents: page.contents,
		})
		.collect();

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_info_pages_subscription(connection_id, conn_update_tx.clone())
		.await;

	let message =
		FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminInfoPages(info_pages)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_admin_info_pages_message(
	db_connection: Arc<Mutex<PgConnection>>,
	connection_id: &str,
	user: &SelfUserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminInfoPageUpdate,
) {
	if !user.is_admin {
		return;
	}

	if !subscription_manager
		.lock()
		.await
		.is_subscribed_to_admin_info_pages(connection_id)
		.await
	{
		return;
	}

	match update_message {
		AdminInfoPageUpdate::UpdateInfoPage(info_page) => {
			let info_page: InfoPage =
				{
					let mut db_connection = db_connection.lock().await;
					let db_result: QueryResult<InfoPageDb> = if info_page.id.is_empty() {
						let new_info_page = InfoPageDb {
							id: cuid2::create_id(),
							event: info_page.event.id,
							title: info_page.title,
							contents: info_page.contents,
						};
						diesel::insert_into(info_pages::table)
							.values(new_info_page)
							.get_result(&mut *db_connection)
					} else {
						diesel::update(info_pages::table)
							.filter(info_pages::id.eq(&info_page.id))
							.set((
								info_pages::title.eq(info_page.title),
								info_pages::contents.eq(info_page.contents),
							))
							.get_result(&mut *db_connection)
					};

					match db_result {
						Ok(page) => {
							let event: EventDb =
								match events::table.find(&page.event).first(&mut *db_connection) {
									Ok(event) => event,
									Err(error) => {
										tide::log::error!("Failed to get event associated with info page for admin message broadcast: {}", error);
										return;
									}
								};
							InfoPage {
								id: page.id,
								event: event.into(),
								title: page.title,
								contents: page.contents,
							}
						}
						Err(error) => {
							tide::log::error!("Failed to update info page for admin update: {}", error);
							return;
						}
					}
				};

			let subscription_manager = subscription_manager.lock().await;
			let event_message = SubscriptionData::EventUpdate(
				info_page.event.clone(),
				Box::new(EventSubscriptionData::UpdateInfoPage(info_page.clone())),
			);
			let send_result = subscription_manager
				.broadcast_event_message(&info_page.event.id, event_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to broadcast event update for info page: {}", error);
			}

			let admin_message = SubscriptionData::AdminInfoPagesUpdate(AdminInfoPageData::UpdateInfoPage(info_page));
			let send_result = subscription_manager
				.broadcast_admin_info_pages_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to broadcast admin update for info page: {}", error);
			}
		}
		AdminInfoPageUpdate::DeleteInfoPage(info_page) => {
			let delete_result: QueryResult<EventDb> = {
				let mut db_connection = db_connection.lock().await;
				db_connection.transaction(|db_connection| {
					let page: InfoPageDb = diesel::delete(info_pages::table)
						.filter(info_pages::id.eq(&info_page.id))
						.get_result(db_connection)?;
					let event: EventDb = events::table.find(&page.event).first(db_connection)?;
					Ok(event)
				})
			};
			let event: Event = match delete_result {
				Ok(event) => event.into(),
				Err(error) => {
					tide::log::error!("An error occurred deleting an info page: {}", error);
					return;
				}
			};

			let subscription_manager = subscription_manager.lock().await;
			let event_message = SubscriptionData::EventUpdate(
				event.clone(),
				Box::new(EventSubscriptionData::DeleteInfoPage(info_page.clone())),
			);
			let send_result = subscription_manager
				.broadcast_event_message(&event.id, event_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to broadcast event update for info page deletion: {}", error);
			}

			let admin_message = SubscriptionData::AdminInfoPagesUpdate(AdminInfoPageData::DeleteInfoPage(info_page));
			let send_result = subscription_manager
				.broadcast_admin_info_pages_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to broadcast admin update for info page deletion: {}", error);
			}
		}
	}
}
