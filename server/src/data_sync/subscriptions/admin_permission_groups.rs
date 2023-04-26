use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::{PermissionEvent, PermissionGroup as PermissionGroupDb, User, UserPermission};
use crate::schema::{permission_events, permission_groups, user_permissions, users};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{
	PermissionGroup, PermissionGroupEventAssociation, UserPermissionGroupAssociation,
};
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

pub async fn subscribe_to_admin_permission_groups_users(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	let mut db_connection = db_connection.lock().await;
	let permission_group_users: QueryResult<Vec<UserPermission>> = user_permissions::table.load(&mut *db_connection);

	let permission_group_users = match permission_group_users {
		Ok(data) => data,
		Err(error) => {
			tide::log::error!(
				"A database error occurred getting permission group user data for the admin subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminPermissionGroupUsers,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};
	let user_ids: Vec<String> = permission_group_users
		.iter()
		.map(|association| association.user_id.clone())
		.collect();
	let group_ids: Vec<String> = permission_group_users
		.iter()
		.map(|association| association.permission_group.clone())
		.collect();
	let users: QueryResult<Vec<User>> = users::table
		.filter(users::id.eq_any(&user_ids))
		.load(&mut *db_connection);
	let groups: QueryResult<Vec<PermissionGroupDb>> = permission_groups::table
		.filter(permission_groups::id.eq_any(&group_ids))
		.load(&mut *db_connection);

	let users = match users {
		Ok(mut users) => {
			let user_map: HashMap<String, UserData> =
				users.drain(..).map(|user| (user.id.clone(), user.into())).collect();
			user_map
		}
		Err(error) => {
			tide::log::error!(
				"A database error occurred getting users for permission group user admin data: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminPermissionGroupUsers,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};
	let permission_groups = match groups {
		Ok(mut groups) => {
			let group_map: HashMap<String, PermissionGroup> =
				groups.drain(..).map(|group| (group.id.clone(), group.into())).collect();
			group_map
		}
		Err(error) => {
			tide::log::error!(
				"A database error occurred getting groups for permission group user admin data: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminPermissionGroupUsers,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let mut permission_group_user_associations: Vec<UserPermissionGroupAssociation> =
		Vec::with_capacity(permission_group_users.len());
	for permission_group_user in permission_group_users.iter() {
		let user = users.get(&permission_group_user.user_id).unwrap().clone();
		let permission_group = permission_groups
			.get(&permission_group_user.permission_group)
			.unwrap()
			.clone();
		permission_group_user_associations.push(UserPermissionGroupAssociation { user, permission_group });
	}

	let mut subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_permission_group_users_subscription(user, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(
		InitialSubscriptionLoadData::AdminPermissionGroupUsers(permission_group_user_associations),
	));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}
