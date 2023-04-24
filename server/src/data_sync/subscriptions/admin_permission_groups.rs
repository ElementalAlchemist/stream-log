use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::{Event as EventDb, PermissionEvent, PermissionGroup as PermissionGroupDb};
use crate::schema::{events, permission_events, permission_groups};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use stream_log_shared::messages::admin::{PermissionGroup, PermissionGroupEventAssociation};
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_permission_groups(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	let mut db_connection = db_connection.lock().await;
	let permission_groups: QueryResult<Vec<PermissionGroupDb>> = permission_groups::table.load(&mut *db_connection);
	let permission_groups: Vec<PermissionGroup> = match permission_groups {
		Ok(mut permission_groups) => permission_groups.drain(..).map(|group| group.into()).collect(),
		Err(error) => {
			tide::log::error!("A database error occurred getting the permission groups for an admin permission groups subscription: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminPermissionGroups,
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
		.add_admin_permission_group_subscription(user, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(
		InitialSubscriptionLoadData::AdminPermissionGroups(permission_groups),
	));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn subscribe_to_admin_permission_groups_events(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	let mut db_connection = db_connection.lock().await;
	let permission_group_events: QueryResult<Vec<PermissionEvent>> = permission_events::table.load(&mut *db_connection);

	let mut permission_group_events = match permission_group_events {
		Ok(data) => data,
		Err(error) => {
			tide::log::error!(
				"A database error occurred getting permission group event data for the admin subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminPermissionGroupEvents,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};
	let permission_group_events: Vec<PermissionGroupEventAssociation> = permission_group_events
		.drain(..)
		.map(|association| association.into())
		.collect();

	let mut subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_permission_group_events_subscription(user, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(
		InitialSubscriptionLoadData::AdminPermissionGroupEvents(permission_group_events),
	));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}
