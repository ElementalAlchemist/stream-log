use crate::data_sync::user::UserDataUpdate;
use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::{
	Event as EventDb, Permission, PermissionEvent, PermissionGroup as PermissionGroupDb, User, UserPermission,
};
use crate::schema::{events, permission_events, permission_groups, user_permissions, users};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{
	AdminPermissionGroupData, AdminPermissionGroupUpdate, AdminUserPermissionGroupData, AdminUserPermissionGroupUpdate,
	PermissionGroup, PermissionGroupEventAssociation, UserPermissionGroupAssociation,
};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_permission_groups(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message = FromServerMessage::SubscriptionFailure(
			SubscriptionType::AdminPermissionGroups,
			SubscriptionFailureInfo::NotAllowed,
		);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let (permission_groups, permission_group_events) = {
		let mut db_connection = db_connection.lock().await;
		let permission_groups: QueryResult<Vec<PermissionGroupDb>> = permission_groups::table.load(&mut *db_connection);
		let permission_group_events: QueryResult<Vec<PermissionEvent>> =
			permission_events::table.load(&mut *db_connection);
		(permission_groups, permission_group_events)
	};
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
	let permission_group_events: Vec<PermissionGroupEventAssociation> = match permission_group_events {
		Ok(mut group_events) => group_events.drain(..).map(|association| association.into()).collect(),
		Err(error) => {
			tide::log::error!("A database error occurred getting the permission group and event associations for an admin permission groups subscription: {}", error);
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

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_permission_group_subscription(connection_id, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(
		InitialSubscriptionLoadData::AdminPermissionGroups(permission_groups, permission_group_events),
	));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_admin_permission_groups_message(
	db_connection: Arc<Mutex<PgConnection>>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminPermissionGroupUpdate,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.is_subscribed_to_admin_permission_groups(connection_id)
		.await
	{
		return;
	}

	match update_message {
		AdminPermissionGroupUpdate::UpdateGroup(mut group) => {
			{
				let mut db_connection = db_connection.lock().await;
				if group.id.is_empty() {
					group.id = cuid2::create_id();
					let group_db = PermissionGroupDb {
						id: group.id.clone(),
						name: group.name.clone(),
					};
					let db_result = diesel::insert_into(permission_groups::table)
						.values(group_db)
						.execute(&mut *db_connection);
					if let Err(error) = db_result {
						tide::log::error!("A database error occurred adding a new permission group: {}", error);
						return;
					}
				} else {
					let db_result = diesel::update(permission_groups::table)
						.filter(permission_groups::id.eq(&group.id))
						.set(permission_groups::name.eq(&group.name))
						.execute(&mut *db_connection);
					if let Err(error) = db_result {
						tide::log::error!("A database error occurred updating a permission group: {}", error);
						return;
					}
				}
			}

			let subscription_manager = subscription_manager.lock().await;
			let message = SubscriptionData::AdminPermissionGroupsUpdate(AdminPermissionGroupData::UpdateGroup(group));
			let send_result = subscription_manager
				.broadcast_admin_permission_groups_message(message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send admin permission group update: {}", error);
			}
		}
		AdminPermissionGroupUpdate::SetEventPermissionForGroup(event_group_association) => {
			let (user_permissions, event) = {
				let mut db_connection = db_connection.lock().await;
				let permission_event = PermissionEvent {
					permission_group: event_group_association.group.clone(),
					event: event_group_association.event.clone(),
					level: event_group_association.permission.into(),
				};
				let db_result = diesel::insert_into(permission_events::table)
					.values(&permission_event)
					.on_conflict((permission_events::permission_group, permission_events::event))
					.do_update()
					.set(permission_events::level.eq(permission_event.level))
					.execute(&mut *db_connection);
				if let Err(error) = db_result {
					tide::log::error!(
						"A database error occurred setting permissions for an event in a permission group: {}",
						error
					);
					return;
				}

				// If this update lowered the event's permissions in this group, each user's other groups *might* have a higher permission level for the event.
				let user_permissions: QueryResult<Vec<(String, Option<Permission>)>> = user_permissions::table
					.filter(user_permissions::permission_group.eq(&event_group_association.group))
					.left_outer_join(
						permission_events::table
							.on(user_permissions::permission_group.eq(permission_events::permission_group)),
					)
					.filter(permission_events::event.eq(&event_group_association.event))
					.select((user_permissions::user_id, permission_events::level.nullable()))
					.load(&mut *db_connection);
				let event: QueryResult<EventDb> = events::table
					.find(&event_group_association.event)
					.first(&mut *db_connection);
				let event =
					match event {
						Ok(event) => event,
						Err(error) => {
							tide::log::error!("A database error occurred getting the event associated with a permission group update: {}", error);
							return;
						}
					};

				(user_permissions, event)
			};

			let mut subscription_manager = subscription_manager.lock().await;
			let admin_message = SubscriptionData::AdminPermissionGroupsUpdate(
				AdminPermissionGroupData::SetEventPermissionForGroup(event_group_association),
			);
			let send_result = subscription_manager
				.broadcast_admin_permission_groups_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send permission group events update admin message: {}", error);
			}

			match user_permissions {
				Ok(users) => {
					for (user, permission) in users {
						let message = UserDataUpdate::EventPermissions(event.clone().into(), permission);
						subscription_manager.send_message_to_user(&user, message).await;
					}
				}
				Err(error) => tide::log::error!(
					"Failed to get users and their permissions associated with a permission group and event: {}",
					error
				),
			}
		}
		AdminPermissionGroupUpdate::RemoveEventFromGroup(group, event) => {
			let user_permissions: QueryResult<Vec<(String, Option<Permission>)>> = {
				let mut db_connection = db_connection.lock().await;
				let db_result = diesel::delete(permission_events::table)
					.filter(
						permission_events::permission_group
							.eq(&group.id)
							.and(permission_events::event.eq(&event.id)),
					)
					.execute(&mut *db_connection);
				if let Err(error) = db_result {
					tide::log::error!(
						"A database error occurred removing an event from a permission group: {}",
						error
					);
					return;
				}

				user_permissions::table
					.filter(user_permissions::permission_group.eq(&event.id))
					.left_outer_join(
						permission_events::table
							.on(user_permissions::permission_group.eq(permission_events::permission_group)),
					)
					.filter(permission_events::event.eq(&event.id))
					.select((user_permissions::user_id, permission_events::level.nullable()))
					.load(&mut *db_connection)
			};

			let mut subscription_manager = subscription_manager.lock().await;
			let admin_message = SubscriptionData::AdminPermissionGroupsUpdate(
				AdminPermissionGroupData::RemoveEventFromGroup(group.clone(), event.clone()),
			);
			let send_result = subscription_manager
				.broadcast_admin_permission_groups_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!(
					"Failed to send message to remove event from permission group: {}",
					error
				);
			}

			match user_permissions {
				Ok(users) => {
					for (user, permission) in users {
						let message = UserDataUpdate::EventPermissions(event.clone(), permission);
						subscription_manager.send_message_to_user(&user, message).await;
					}
				}
				Err(error) => tide::log::error!(
					"Failed to get users and their permissions associated with a permission group and event: {}",
					error
				),
			}
		}
	};
}

pub async fn subscribe_to_admin_permission_groups_users(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message = FromServerMessage::SubscriptionFailure(
			SubscriptionType::AdminPermissionGroupUsers,
			SubscriptionFailureInfo::NotAllowed,
		);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

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

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_permission_group_users_subscription(connection_id, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(
		InitialSubscriptionLoadData::AdminPermissionGroupUsers(permission_group_user_associations),
	));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_admin_permission_group_users_message(
	db_connection: Arc<Mutex<PgConnection>>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminUserPermissionGroupUpdate,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.is_subscribed_to_admin_permission_group_users(connection_id)
		.await
	{
		return;
	}

	match update_message {
		AdminUserPermissionGroupUpdate::AddUserToGroup(user_group_association) => {
			let user_event_permissions = {
				let mut db_connection = db_connection.lock().await;

				let user_event_permissions: QueryResult<Vec<(Event, Option<Permission>)>> =
					db_connection.transaction(|db_connection| {
						let user_permission = UserPermission {
							user_id: user_group_association.user.id.clone(),
							permission_group: user_group_association.permission_group.id.clone(),
						};
						diesel::insert_into(user_permissions::table)
							.values(user_permission)
							.execute(&mut *db_connection)?;

						let affected_event_permissions: Vec<PermissionEvent> = permission_events::table
							.filter(permission_events::permission_group.eq(&user_group_association.permission_group.id))
							.load(db_connection)?;
						let affected_event_ids: Vec<String> = affected_event_permissions
							.iter()
							.map(|event_permission| event_permission.event.clone())
							.collect();
						let mut affected_events: Vec<EventDb> = events::table
							.filter(events::id.eq_any(&affected_event_ids))
							.load(db_connection)?;
						let affected_events: Vec<Event> = affected_events.drain(..).map(|event| event.into()).collect();

						let all_user_event_permissions: Vec<PermissionEvent> = permission_events::table
							.filter(
								user_permissions::table
									.filter(user_permissions::user_id.eq(&user_group_association.user.id).and(
										user_permissions::permission_group.eq(permission_events::permission_group),
									))
									.count()
									.single_value()
									.gt(0),
							)
							.load(db_connection)?;
						let mut user_event_permissions_by_event: HashMap<String, Vec<PermissionEvent>> = HashMap::new();
						for user_event_permission in all_user_event_permissions {
							user_event_permissions_by_event
								.entry(user_event_permission.event.clone())
								.or_default()
								.push(user_event_permission);
						}

						let mut user_event_permissions = Vec::new();
						for event in affected_events {
							match user_event_permissions_by_event.get(&event.id) {
								Some(event_permissions) => {
									let mut highest_permission_level: Option<Permission> = None;
									for event_permission in event_permissions.iter() {
										match (event_permission.level, highest_permission_level) {
											(Permission::Supervisor, _) => {
												highest_permission_level = Some(Permission::Supervisor);
												break;
											}
											(Permission::Edit, Some(Permission::Supervisor)) => (),
											(Permission::Edit, _) => highest_permission_level = Some(Permission::Edit),
											(Permission::View, Some(Permission::Supervisor)) => (),
											(Permission::View, Some(Permission::Edit)) => (),
											(Permission::View, _) => highest_permission_level = Some(Permission::View),
										}
									}
									user_event_permissions.push((event, highest_permission_level));
								}
								None => user_event_permissions.push((event, None)),
							}
						}

						Ok(user_event_permissions)
					});

				match user_event_permissions {
					Ok(data) => data,
					Err(error) => {
						tide::log::error!(
							"A database error occurred adding a user to a permission group: {}",
							error
						);
						return;
					}
				}
			};

			let mut subscription_manager = subscription_manager.lock().await;
			for (event, permission) in user_event_permissions {
				let user_message = UserDataUpdate::EventPermissions(event, permission);
				subscription_manager
					.send_message_to_user(&user_group_association.user.id, user_message)
					.await;
			}
			let admin_message = SubscriptionData::AdminUserPermissionGroupsUpdate(
				AdminUserPermissionGroupData::AddUserToGroup(user_group_association),
			);
			let send_result = subscription_manager
				.broadcast_admin_permission_group_users_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!(
					"Failed to broadcast permission group user addition to admin subscription: {}",
					error
				);
			}
		}
		AdminUserPermissionGroupUpdate::RemoveUserFromGroup(user_group_association) => {
			let user_event_permissions = {
				let mut db_connection = db_connection.lock().await;

				let user_event_permissions: QueryResult<Vec<(Event, Option<Permission>)>> =
					db_connection.transaction(|db_connection| {
						diesel::delete(user_permissions::table)
							.filter(user_permissions::user_id.eq(&user_group_association.user.id).and(
								user_permissions::permission_group.eq(&user_group_association.permission_group.id),
							))
							.execute(db_connection)?;

						let affected_event_permissions: Vec<PermissionEvent> = permission_events::table
							.filter(permission_events::permission_group.eq(&user_group_association.permission_group.id))
							.load(db_connection)?;
						let affected_event_ids: Vec<String> = affected_event_permissions
							.iter()
							.map(|event_permission| event_permission.event.clone())
							.collect();
						let mut affected_events: Vec<EventDb> = events::table
							.filter(events::id.eq_any(&affected_event_ids))
							.load(db_connection)?;
						let affected_events: Vec<Event> = affected_events.drain(..).map(|event| event.into()).collect();

						let all_user_event_permissions: Vec<PermissionEvent> = permission_events::table
							.filter(
								user_permissions::table
									.filter(user_permissions::user_id.eq(&user_group_association.user.id).and(
										user_permissions::permission_group.eq(permission_events::permission_group),
									))
									.count()
									.single_value()
									.gt(0),
							)
							.load(db_connection)?;
						let mut user_event_permissions_by_event: HashMap<String, Vec<PermissionEvent>> = HashMap::new();
						for user_event_permission in all_user_event_permissions {
							user_event_permissions_by_event
								.entry(user_event_permission.event.clone())
								.or_default()
								.push(user_event_permission);
						}

						let mut user_event_permissions = Vec::new();
						for event in affected_events {
							match user_event_permissions_by_event.get(&event.id) {
								Some(event_permissions) => {
									let mut highest_permission_level: Option<Permission> = None;
									for event_permission in event_permissions.iter() {
										match (event_permission.level, highest_permission_level) {
											(Permission::Supervisor, _) => {
												highest_permission_level = Some(Permission::Supervisor)
											}
											(Permission::Edit, Some(Permission::Supervisor)) => (),
											(Permission::Edit, _) => highest_permission_level = Some(Permission::Edit),
											(Permission::View, Some(Permission::Supervisor)) => (),
											(Permission::View, Some(Permission::Edit)) => (),
											(Permission::View, _) => highest_permission_level = Some(Permission::View),
										}
									}
									user_event_permissions.push((event, highest_permission_level));
								}
								None => user_event_permissions.push((event, None)),
							}
						}

						Ok(user_event_permissions)
					});

				match user_event_permissions {
					Ok(data) => data,
					Err(error) => {
						tide::log::error!(
							"A database error occurred removing a user from a permission group: {}",
							error
						);
						return;
					}
				}
			};

			let mut subscription_manager = subscription_manager.lock().await;
			for (event, permission) in user_event_permissions {
				let user_message = UserDataUpdate::EventPermissions(event, permission);
				subscription_manager
					.send_message_to_user(&user_group_association.user.id, user_message)
					.await;
			}
			let admin_message = SubscriptionData::AdminUserPermissionGroupsUpdate(
				AdminUserPermissionGroupData::RemoveUserFromGroup(user_group_association),
			);
			let send_result = subscription_manager
				.broadcast_admin_permission_group_users_message(admin_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!(
					"Failed to broadcast permission group user removal to admin subscription: {}",
					error
				);
			}
		}
	}
}
