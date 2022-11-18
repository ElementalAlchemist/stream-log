use super::HandleConnectionError;
use crate::models::{
	Event as EventDb, Permission, PermissionEvent, PermissionGroup as PermissionGroupDb, User, UserPermission,
};
use crate::schema::{events, permission_events, permission_groups, user_permissions, users};
use async_std::sync::{Arc, Mutex};
use chrono::prelude::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as QueryError};
use std::collections::HashMap;
use stream_log_shared::messages::admin::{
	AdminAction, EventPermission, PermissionGroup, PermissionGroupWithEvents, UserDataPermissions,
};
use stream_log_shared::messages::events::Event as EventWs;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, DataMessage};
use tide_websockets::WebSocketConnection;

type DestructuredEventPermission = (
	String,
	String,
	Option<String>,
	Option<String>,
	Option<DateTime<Utc>>,
	Option<Permission>,
);

fn generate_cuid_in_transaction() -> Result<String, QueryError> {
	match cuid::cuid() {
		Ok(id) => Ok(id),
		Err(error) => {
			tide::log::error!("Failed to generate CUID: {}", error);
			Err(QueryError::RollbackTransaction)
		}
	}
}

/// Handles administration actions performed by the client
pub async fn handle_admin(
	stream: &mut WebSocketConnection,
	db_connection: Arc<Mutex<PgConnection>>,
	user: &User,
	action: AdminAction,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		return Ok(());
	}
	match action {
		AdminAction::ListEvents => {
			let events: QueryResult<Vec<EventDb>> = {
				let mut db_connection = db_connection.lock().await;
				events::table.load(&mut *db_connection)
			};
			let events: Vec<EventWs> = match events {
				Ok(mut events) => events
					.drain(..)
					.map(|event| EventWs {
						id: event.id,
						name: event.name,
						start_time: event.start_time,
					})
					.collect(),
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					let message: DataMessage<Vec<EventWs>> = DataMessage::Err(DataError::DatabaseError);
					stream.send_json(&message).await?;
					return Err(HandleConnectionError::ConnectionClosed);
				}
			};
			let message = DataMessage::Ok(events);
			stream.send_json(&message).await?;
		}
		AdminAction::EditEvents(events) => {
			let update_result = {
				let mut db_connection = db_connection.lock().await;
				let tx_result: QueryResult<()> = db_connection.transaction(|db_connection| {
					let mut new_events: Vec<EventDb> = Vec::new();
					for event in events.iter() {
						if event.id.is_empty() {
							let id = generate_cuid_in_transaction()?;
							let event = EventDb {
								id,
								name: event.name.clone(),
								start_time: event.start_time,
							};
							new_events.push(event);
						} else {
							diesel::update(events::table.filter(events::id.eq(&event.id)))
								.set((events::name.eq(&event.name), events::start_time.eq(&event.start_time)))
								.execute(&mut *db_connection)?;
						}
					}
					if !new_events.is_empty() {
						diesel::insert_into(events::table)
							.values(&new_events)
							.execute(&mut *db_connection)?;
					}
					Ok(())
				});
				tx_result
			};
			if let Err(error) = update_result {
				tide::log::error!("Database error: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
		AdminAction::ListPermissionGroups => {
			let group_data: QueryResult<Vec<DestructuredEventPermission>> = {
				let mut db_connection = db_connection.lock().await;
				let events_data_table = permission_events::table.inner_join(events::table);
				permission_groups::table
					.left_outer_join(events_data_table)
					.select((
						permission_groups::id,
						permission_groups::name,
						permission_events::event.nullable(),
						events::name.nullable(),
						events::start_time.nullable(),
						permission_events::level.nullable(),
					))
					.load(&mut *db_connection)
			};
			let group_data: Vec<PermissionGroupWithEvents> = match group_data {
				Ok(groups) => {
					let mut permission_group_events: HashMap<PermissionGroup, Vec<EventPermission>> = HashMap::new();
					for (group_id, group_name, event_id, event_name, event_start_time, event_permission) in groups {
						let permission_group = PermissionGroup {
							id: group_id,
							name: group_name,
						};
						let group_entry = permission_group_events.entry(permission_group).or_default();
						if let Some(event_id) = event_id {
							let event_name = event_name.unwrap();
							let event_start_time = event_start_time.unwrap();
							let event_permission = event_permission.unwrap();
							let event = EventWs {
								id: event_id,
								name: event_name,
								start_time: event_start_time,
							};
							let event_permission_data = EventPermission {
								event,
								level: event_permission.into(),
							};
							group_entry.push(event_permission_data);
						}
					}
					permission_group_events
						.drain()
						.map(|(group, events)| PermissionGroupWithEvents { group, events })
						.collect()
				}
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					let message: DataMessage<Vec<PermissionGroupWithEvents>> =
						DataMessage::Err(DataError::DatabaseError);
					stream.send_json(&message).await?;
					return Err(HandleConnectionError::ConnectionClosed);
				}
			};
			let message: DataMessage<Vec<PermissionGroupWithEvents>> = DataMessage::Ok(group_data);
			stream.send_json(&message).await?;
		}
		AdminAction::UpdatePermissionGroups(group_changes) => {
			let mut db_connection = db_connection.lock().await;
			let tx_result: QueryResult<()> = db_connection.transaction(move |db_connection| {
				for group_event_data in group_changes.iter() {
					let id = if group_event_data.group.id.is_empty() {
						let new_id = generate_cuid_in_transaction()?;
						let new_group = PermissionGroupDb {
							id: new_id.clone(),
							name: group_event_data.group.name.clone(),
						};
						diesel::insert_into(permission_groups::table)
							.values(new_group)
							.execute(db_connection)?;
						new_id
					} else {
						diesel::update(permission_groups::table)
							.filter(permission_groups::id.eq(&group_event_data.group.id))
							.set(permission_groups::name.eq(&group_event_data.group.name))
							.execute(db_connection)?;
						group_event_data.group.id.clone()
					};
					diesel::delete(permission_events::table.filter(permission_events::permission_group.eq(&id)))
						.execute(db_connection)?;
					let mut new_event_permissions: Vec<PermissionEvent> = Vec::new();
					for event in group_event_data.events.iter() {
						let group_event_permission = PermissionEvent {
							permission_group: id.clone(),
							event: event.event.id.clone(),
							level: event.level.into(),
						};
						new_event_permissions.push(group_event_permission);
					}
					diesel::insert_into(permission_events::table)
						.values(new_event_permissions)
						.execute(db_connection)?;
				}
				Ok(())
			});
			if let Err(error) = tx_result {
				tide::log::error!("Database error: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
		AdminAction::AddUserToPermissionGroup(permission_group_user) => {
			let mut db_connection = db_connection.lock().await;
			let new_user_permission = UserPermission {
				user_id: permission_group_user.user.id,
				permission_group: permission_group_user.group.id,
			};
			let insert_result = diesel::insert_into(user_permissions::table)
				.values(new_user_permission)
				.execute(&mut *db_connection);
			if let Err(error) = insert_result {
				if let diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) = error {
					// This one might happen sometimes (e.g. race condition from multiple administrators), and it's OK if it does
				} else {
					tide::log::error!("Database error: {}", error);
					return Err(HandleConnectionError::ConnectionClosed);
				}
			}
		}
		AdminAction::RemoveUserFromPermissionGroup(permission_group_user) => {
			let mut db_connection = db_connection.lock().await;
			let delete_result = diesel::delete(
				user_permissions::table.filter(
					user_permissions::user_id
						.eq(&permission_group_user.user.id)
						.and(user_permissions::permission_group.eq(&permission_group_user.group.id)),
				),
			)
			.execute(&mut *db_connection);
			if let Err(error) = delete_result {
				// This one might happen sometimes (e.g. race condition from multiple administrators), and it's OK if it does
				if error != diesel::result::Error::NotFound {
					tide::log::error!("Database error: {}", error);
					return Err(HandleConnectionError::ConnectionClosed);
				}
			}
		}
		AdminAction::ListUsers => {
			let mut db_connection = db_connection.lock().await;
			let lookup_result: QueryResult<Vec<User>> = users::table.load(&mut *db_connection);
			let message = match lookup_result {
				Ok(user_list) => {
					let user_list: Vec<UserData> = user_list
						.iter()
						.map(|user| UserData {
							id: user.id.clone(),
							username: user.name.clone(),
							is_admin: user.is_admin,
						})
						.collect();
					let message: DataMessage<Vec<UserData>> = Ok(user_list);
					message
				}
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					Err(DataError::DatabaseError)
				}
			};
			stream.send_json(&message).await?;
		}
		AdminAction::EditUsers(modified_users) => {
			let mut db_connection = db_connection.lock().await;
			let tx_result: QueryResult<()> = db_connection.transaction(|db_connection| {
				for user in modified_users.iter() {
					diesel::update(users::table)
						.filter(users::id.eq(&user.id))
						.set(users::is_admin.eq(user.is_admin))
						.execute(db_connection)?;
				}
				Ok(())
			});
			if let Err(error) = tx_result {
				tide::log::error!("Database error: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
		AdminAction::ListUserPermissions => {
			let user_groups_table = user_permissions::table.inner_join(permission_groups::table);
			let user_list: QueryResult<Vec<(UserData, Option<PermissionGroup>)>> = {
				let mut db_connection = db_connection.lock().await;
				let lookup_result: QueryResult<Vec<(_, _, _, Option<String>, Option<String>)>> = users::table
					.left_outer_join(user_groups_table)
					.select((
						users::id,
						users::name,
						users::is_admin,
						permission_groups::id.nullable(),
						permission_groups::name.nullable(),
					))
					.load(&mut *db_connection);
				match lookup_result {
					Ok(results) => {
						let mut result_list: Vec<(UserData, Option<PermissionGroup>)> =
							Vec::with_capacity(results.len());
						for (user_id, user_name, user_is_admin, group_id, group_name) in results {
							let user_data = UserData {
								id: user_id,
								username: user_name,
								is_admin: user_is_admin,
							};
							let permission_group = if let Some(group_id) = group_id {
								let group_name = group_name.unwrap();
								Some(PermissionGroup {
									id: group_id,
									name: group_name,
								})
							} else {
								None
							};
							result_list.push((user_data, permission_group));
						}
						Ok(result_list)
					}
					Err(error) => Err(error),
				}
			};
			let message: DataMessage<Vec<UserDataPermissions>> = match user_list {
				Ok(user_list) => {
					let mut user_groups: HashMap<UserData, Vec<PermissionGroup>> = HashMap::new();
					for (user_data, group) in user_list {
						let user_group_entry = user_groups.entry(user_data).or_default();
						if let Some(group_data) = group {
							user_group_entry.push(group_data);
						}
					}
					let user_groups = user_groups
						.drain()
						.map(|(user_data, group_data)| UserDataPermissions {
							user: user_data,
							groups: group_data,
						})
						.collect();
					DataMessage::Ok(user_groups)
				}
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					DataMessage::Err(DataError::DatabaseError)
				}
			};
			stream.send_json(&message).await?;
		}
	}

	Ok(())
}
