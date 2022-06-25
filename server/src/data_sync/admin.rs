use super::HandleConnectionError;
use crate::models::{
	Event as EventDb, Permission, PermissionEvent, PermissionGroup as PermissionGroupDb, User, UserPermission,
};
use crate::schema::{events, permission_events, permission_groups, user_permissions, users};
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{
	AdminAction, EventPermission, PermissionGroup, PermissionGroupEvent, PermissionGroupWithEvents, UserDataPermissions,
};
use stream_log_shared::messages::events::Event as EventWs;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, DataMessage};
use tide_websockets::WebSocketConnection;

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
				let db_connection = db_connection.lock().await;
				events::table.load(&*db_connection)
			};
			let events: Vec<EventWs> = match events {
				Ok(mut events) => events
					.drain(..)
					.map(|event| EventWs {
						id: event.id,
						name: event.name,
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
		AdminAction::AddEvent(new_event) => {
			let id = match cuid::cuid() {
				Ok(id) => id,
				Err(error) => {
					tide::log::error!("Failed to generate CUID: {}", error);
					return Err(HandleConnectionError::ConnectionClosed);
				}
			};
			let event = EventDb {
				id,
				name: new_event.name,
			};
			let insert_result = {
				let db_connection = db_connection.lock().await;
				diesel::insert_into(events::table)
					.values(&event)
					.execute(&*db_connection)
			};
			if let Err(error) = insert_result {
				tide::log::error!("Database error: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
		AdminAction::EditEvent(event) => {
			let update_result = {
				let db_connection = db_connection.lock().await;
				diesel::update(events::table.filter(events::id.eq(event.id)))
					.set(events::name.eq(event.name))
					.execute(&*db_connection)
			};
			if let Err(error) = update_result {
				tide::log::error!("Database error: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
		AdminAction::ListPermissionGroups => {
			let group_data: QueryResult<Vec<(_, _, _, Option<String>, Option<Permission>)>> = {
				let db_connection = db_connection.lock().await;
				let events_data_table = permission_events::table.inner_join(events::table);
				permission_groups::table
					.left_outer_join(events_data_table)
					.select((
						permission_groups::id,
						permission_groups::name,
						permission_events::event.nullable(),
						events::name.nullable(),
						permission_events::level.nullable(),
					))
					.load(&*db_connection)
			};
			let group_data: Vec<PermissionGroupWithEvents> = match group_data {
				Ok(groups) => {
					let mut permission_group_events: HashMap<PermissionGroup, Vec<EventPermission>> = HashMap::new();
					for (group_id, group_name, event_id, event_name, event_permission) in groups {
						let permission_group = PermissionGroup {
							id: group_id,
							name: group_name,
						};
						let group_entry = permission_group_events.entry(permission_group).or_default();
						if let Some(event_id) = event_id {
							let event_name = event_name.unwrap();
							let event_permission = event_permission.unwrap();
							let event = EventWs {
								id: event_id,
								name: event_name,
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
		AdminAction::CreatePermissionGroup(group_name) => {
			let db_connection = db_connection.lock().await;
			let id = match cuid::cuid() {
				Ok(id) => id,
				Err(error) => {
					tide::log::error!("Failed to generate CUID: {}", error);
					return Err(HandleConnectionError::ConnectionClosed);
				}
			};
			let new_group = PermissionGroupDb { id, name: group_name };
			let insert_result = diesel::insert_into(permission_groups::table)
				.values(new_group)
				.execute(&*db_connection);
			if let Err(error) = insert_result {
				if let diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) = error {
					// This one might happen sometimes (e.g. race condition from multiple administrators), and it's OK if it does
				} else {
					tide::log::error!("Database error: {}", error);
					return Err(HandleConnectionError::ConnectionClosed);
				}
			}
		}
		AdminAction::SetEventViewForGroup(permission_group_event) => {
			update_permission_event(&db_connection, permission_group_event, Permission::View).await?;
		}
		AdminAction::SetEventEditForGroup(permission_group_event) => {
			update_permission_event(&db_connection, permission_group_event, Permission::Edit).await?;
		}
		AdminAction::RemoveEventFromGroup(permission_group_event) => {
			let db_connection = db_connection.lock().await;
			let delete_result = diesel::delete(
				permission_events::table.filter(
					permission_events::permission_group
						.eq(&permission_group_event.group.id)
						.and(permission_events::event.eq(&permission_group_event.event.id)),
				),
			)
			.execute(&*db_connection);
			if let Err(error) = delete_result {
				// This one might happen sometimes (e.g. race condition from multiple administrators), and it's OK if it does
				if error != diesel::result::Error::NotFound {
					tide::log::error!("Database error: {}", error);
					return Err(HandleConnectionError::ConnectionClosed);
				}
			}
		}
		AdminAction::AddUserToPermissionGroup(permission_group_user) => {
			let db_connection = db_connection.lock().await;
			let new_user_permission = UserPermission {
				user_id: permission_group_user.user.id,
				permission_group: permission_group_user.group.id,
			};
			let insert_result = diesel::insert_into(user_permissions::table)
				.values(new_user_permission)
				.execute(&*db_connection);
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
			let db_connection = db_connection.lock().await;
			let delete_result = diesel::delete(
				user_permissions::table.filter(
					user_permissions::user_id
						.eq(&permission_group_user.user.id)
						.and(user_permissions::permission_group.eq(&permission_group_user.group.id)),
				),
			)
			.execute(&*db_connection);
			if let Err(error) = delete_result {
				// This one might happen sometimes (e.g. race condition from multiple administrators), and it's OK if it does
				if error != diesel::result::Error::NotFound {
					tide::log::error!("Database error: {}", error);
					return Err(HandleConnectionError::ConnectionClosed);
				}
			}
		}
		AdminAction::ListUsers => {
			let user_groups_table = user_permissions::table.inner_join(permission_groups::table);
			let user_list: QueryResult<Vec<(UserData, Option<PermissionGroup>)>> = {
				let db_connection = db_connection.lock().await;
				let lookup_result: QueryResult<Vec<(_, _, _, Option<String>, Option<String>)>> = users::table
					.left_outer_join(user_groups_table)
					.select((
						users::id,
						users::name,
						users::is_admin,
						permission_groups::id.nullable(),
						permission_groups::name.nullable(),
					))
					.load(&*db_connection);
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

async fn update_permission_event(
	db_connection: &Arc<Mutex<PgConnection>>,
	permission_group_event: PermissionGroupEvent,
	permission: Permission,
) -> Result<(), HandleConnectionError> {
	let db_connection = db_connection.lock().await;
	let tx_result: QueryResult<()> = db_connection.transaction(|| {
		let existing_record: Option<PermissionEvent> = permission_events::table
			.filter(
				permission_events::permission_group
					.eq(&permission_group_event.group.id)
					.and(permission_events::event.eq(&permission_group_event.event.id)),
			)
			.first(&*db_connection)
			.optional()?;
		if existing_record.is_some() {
			diesel::update(permission_events::table)
				.filter(
					permission_events::permission_group
						.eq(&permission_group_event.group.id)
						.and(permission_events::event.eq(&permission_group_event.event.id)),
				)
				.set(permission_events::level.eq(permission))
				.execute(&*db_connection)?;
		} else {
			let new_record = PermissionEvent {
				permission_group: permission_group_event.group.id.clone(),
				event: permission_group_event.event.id.clone(),
				level: permission,
			};
			diesel::insert_into(permission_events::table)
				.values(new_record)
				.execute(&*db_connection)?;
		}
		Ok(())
	});
	match tx_result {
		Ok(_) => Ok(()),
		Err(error) => {
			tide::log::error!("Database error: {}", error);
			Err(HandleConnectionError::ConnectionClosed)
		}
	}
}
