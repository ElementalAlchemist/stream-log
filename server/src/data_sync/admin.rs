use super::HandleConnectionError;
use crate::models::{
	AvailableEventType, Event as EventDb, EventType as EventTypeDb, Permission, PermissionEvent,
	PermissionGroup as PermissionGroupDb, Tag as TagDb, User, UserPermission,
};
use crate::schema::{
	available_event_types_for_event, event_types, events, permission_events, permission_groups, tags, user_permissions,
	users,
};
use async_std::sync::{Arc, Mutex};
use chrono::prelude::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as QueryError};
use rgb::RGB8;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::admin::{AdminAction, EventPermission, PermissionGroup, PermissionGroupWithEvents};
use stream_log_shared::messages::event_types::EventType;
use stream_log_shared::messages::events::Event as EventWs;
use stream_log_shared::messages::tags::Tag;
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
			let group_data: QueryResult<Vec<PermissionGroupDb>> = {
				let mut db_connection = db_connection.lock().await;
				permission_groups::table.load(&mut *db_connection)
			};
			let message = match group_data {
				Ok(groups) => {
					let send_groups: Vec<PermissionGroup> = groups
						.iter()
						.map(|group| PermissionGroup {
							id: group.id.clone(),
							name: group.name.clone(),
						})
						.collect();
					DataMessage::Ok(send_groups)
				}
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					DataMessage::Err(DataError::DatabaseError)
				}
			};
			stream.send_json(&message).await?;
		}
		AdminAction::ListPermissionGroupsWithEvents => {
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
		AdminAction::ListUserPermissionGroups(user) => {
			let permission_groups_result: QueryResult<Vec<PermissionGroupDb>> = {
				let mut db_connection = db_connection.lock().await;
				permission_groups::table
					.filter(
						user_permissions::table
							.filter(
								user_permissions::user_id
									.eq(&user.id)
									.and(user_permissions::permission_group.eq(permission_groups::id)),
							)
							.count()
							.single_value()
							.gt(0),
					)
					.load(&mut *db_connection)
			};
			let message = match permission_groups_result {
				Ok(mut permission_groups) => {
					let permission_groups: Vec<PermissionGroup> = permission_groups
						.drain(..)
						.map(|group| PermissionGroup {
							id: group.id,
							name: group.name,
						})
						.collect();
					DataMessage::Ok(permission_groups)
				}
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					DataMessage::Err(DataError::DatabaseError)
				}
			};
			stream.send_json(&message).await?;
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
							color: RGB8::new(
								user.color_red.try_into().unwrap(),
								user.color_green.try_into().unwrap(),
								user.color_blue.try_into().unwrap(),
							),
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
					let color_red: i32 = user.color.r.into();
					let color_green: i32 = user.color.g.into();
					let color_blue: i32 = user.color.b.into();
					diesel::update(users::table)
						.filter(users::id.eq(&user.id))
						.set((
							users::is_admin.eq(user.is_admin),
							users::color_red.eq(color_red),
							users::color_green.eq(color_green),
							users::color_blue.eq(color_blue),
						))
						.execute(db_connection)?;
				}
				Ok(())
			});
			if let Err(error) = tx_result {
				tide::log::error!("Database error: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
		AdminAction::ListUsersWithNoPermissionGroups => {
			let users_result: QueryResult<Vec<User>> = {
				let mut db_connection = db_connection.lock().await;
				users::table
					.filter(
						user_permissions::table
							.filter(user_permissions::user_id.eq(users::id))
							.count()
							.single_value()
							.eq(0),
					)
					.load(&mut *db_connection)
			};
			let message = match users_result {
				Ok(mut users) => {
					let user_data: Vec<UserData> = users
						.drain(..)
						.map(|user| UserData {
							id: user.id,
							username: user.name,
							is_admin: user.is_admin,
							color: RGB8::new(
								user.color_red.try_into().unwrap(),
								user.color_green.try_into().unwrap(),
								user.color_blue.try_into().unwrap(),
							),
						})
						.collect();
					DataMessage::Ok(user_data)
				}
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					DataMessage::Err(DataError::DatabaseError)
				}
			};
			stream.send_json(&message).await?;
		}
		AdminAction::ListEventTypes => {
			let event_types: QueryResult<Vec<EventTypeDb>> = {
				let mut db_connection = db_connection.lock().await;
				event_types::table.load(&mut *db_connection)
			};
			let message = match event_types {
				Ok(event_types) => {
					let event_types: Vec<EventType> = event_types
						.iter()
						.map(|et| EventType {
							id: et.id.clone(),
							name: et.name.clone(),
							color: et.color(),
						})
						.collect();
					DataMessage::Ok(event_types)
				}
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					DataMessage::Err(DataError::DatabaseError)
				}
			};
			stream.send_json(&message).await?;
		}
		AdminAction::AddEventType(event_type) => {
			let new_id = match cuid::cuid() {
				Ok(id) => id,
				Err(error) => {
					tide::log::error!("Failed to generate CUID: {}", error);
					return Err(HandleConnectionError::ConnectionClosed);
				}
			};
			let new_event = EventTypeDb {
				id: new_id.clone(),
				name: event_type.name,
				color_red: event_type.color.r.into(),
				color_green: event_type.color.g.into(),
				color_blue: event_type.color.b.into(),
			};

			let mut db_connection = db_connection.lock().await;
			let result: QueryResult<_> = diesel::insert_into(event_types::table)
				.values(&new_event)
				.execute(&mut *db_connection);
			if let Err(error) = result {
				tide::log::error!("Error adding event type: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
			let message = match result {
				Ok(_) => DataMessage::Ok(new_id),
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					DataMessage::Err(DataError::DatabaseError)
				}
			};
			stream.send_json(&message).await?;
		}
		AdminAction::UpdateEventType(event_type) => {
			let mut db_connection = db_connection.lock().await;
			let red: i32 = event_type.color.r.into();
			let green: i32 = event_type.color.g.into();
			let blue: i32 = event_type.color.b.into();

			let result: QueryResult<_> = diesel::update(event_types::table.filter(event_types::id.eq(&event_type.id)))
				.set((
					event_types::name.eq(&event_type.name),
					event_types::color_red.eq(red),
					event_types::color_green.eq(green),
					event_types::color_blue.eq(blue),
				))
				.execute(&mut *db_connection);
			if let Err(error) = result {
				tide::log::error!("Error updating event type: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
		AdminAction::ListEventTypesForEvent(event) => {
			let event_types_result: QueryResult<Vec<EventTypeDb>> = {
				let mut db_connection = db_connection.lock().await;
				event_types::table
					.filter(
						available_event_types_for_event::table
							.filter(
								available_event_types_for_event::event_id
									.eq(&event.id)
									.and(available_event_types_for_event::event_type.eq(event_types::id)),
							)
							.count()
							.single_value()
							.gt(0),
					)
					.load(&mut *db_connection)
			};

			let message = match event_types_result {
				Ok(event_types) => {
					let event_types: Vec<EventType> = event_types
						.iter()
						.map(|et| EventType {
							id: et.id.clone(),
							name: et.name.clone(),
							color: et.color(),
						})
						.collect();
					DataMessage::Ok(event_types)
				}
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					DataMessage::Err(DataError::DatabaseError)
				}
			};
			stream.send_json(&message).await?;
		}
		AdminAction::UpdateEventTypesForEvent(event, event_types) => {
			let mut db_connection = db_connection.lock().await;
			let tx_result: QueryResult<()> = db_connection.transaction(|db_connection| {
				diesel::delete(available_event_types_for_event::table)
					.filter(available_event_types_for_event::event_id.eq(&event.id))
					.execute(&mut *db_connection)?;

				let available_event_types: Vec<AvailableEventType> = event_types
					.iter()
					.map(|et| AvailableEventType {
						event_id: event.id.clone(),
						event_type: et.id.clone(),
					})
					.collect();
				diesel::insert_into(available_event_types_for_event::table)
					.values(available_event_types)
					.execute(&mut *db_connection)?;

				Ok(())
			});
			if let Err(error) = tx_result {
				tide::log::error!("Database error: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
		AdminAction::ListTagsForEvent(event) => {
			let mut db_connection = db_connection.lock().await;
			let tags_result: QueryResult<Vec<TagDb>> = tags::table
				.filter(tags::for_event.eq(&event.id))
				.load(&mut *db_connection);
			let message = match tags_result {
				Ok(tag_list) => {
					let tags: Vec<Tag> = tag_list
						.iter()
						.map(|t| Tag {
							id: t.id.clone(),
							name: t.tag.clone(),
							description: t.description.clone(),
						})
						.collect();
					DataMessage::Ok(tags)
				}
				Err(error) => {
					tide::log::error!("Database error: {}", error);
					DataMessage::Err(DataError::DatabaseError)
				}
			};
			stream.send_json(&message).await?;
		}
		AdminAction::AddTag(mut tag, event) => {
			let new_id = match cuid::cuid() {
				Ok(id) => id,
				Err(error) => {
					tide::log::error!("Failed to generate CUID: {}", error);
					return Err(HandleConnectionError::ConnectionClosed);
				}
			};
			let db_tag = TagDb {
				id: new_id.clone(),
				for_event: event.id,
				tag: tag.name.clone(),
				description: tag.description.clone(),
			};
			tag.id = new_id;

			let mut db_connection = db_connection.lock().await;
			let response = match diesel::insert_into(tags::table)
				.values(db_tag)
				.execute(&mut *db_connection)
			{
				Ok(_) => DataMessage::Ok(tag),
				Err(error) => {
					tide::log::error!("Error adding tag: {}", error);
					DataMessage::Err(DataError::DatabaseError)
				}
			};
			stream.send_json(&response).await?;
		}
		AdminAction::UpdateTagDescription(tag) => {
			let mut db_connection = db_connection.lock().await;
			if let Err(error) = diesel::update(tags::table.filter(tags::id.eq(&tag.id)))
				.set(tags::description.eq(&tag.description))
				.execute(&mut *db_connection)
			{
				tide::log::error!("Database error: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
		AdminAction::RemoveTag(tag) => {
			let mut db_connection = db_connection.lock().await;
			if let Err(error) = diesel::delete(tags::table.filter(tags::id.eq(&tag.id))).execute(&mut *db_connection) {
				tide::log::error!("Error deleting tag: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
		AdminAction::ReplaceTag(old_tag, new_tag) => {
			todo!("The data structures that use these tags don't exist yet");
		}
		AdminAction::CopyTags(from_event, to_event) => {
			let mut db_connection = db_connection.lock().await;
			let tx_result: QueryResult<()> = db_connection.transaction(|db_connection| {
				let from_event_tags: Vec<TagDb> = tags::table
					.filter(tags::for_event.eq(&from_event.id))
					.load(&mut *db_connection)?;
				let to_event_tags: Vec<TagDb> = tags::table
					.filter(tags::for_event.eq(&to_event.id))
					.load(&mut *db_connection)?;
				let to_event_names: HashSet<String> = to_event_tags.iter().map(|tag| tag.tag.clone()).collect();

				let mut new_events: Vec<TagDb> = from_event_tags
					.iter()
					.filter(|tag| !to_event_names.contains(&tag.tag))
					.map(|tag| TagDb {
						id: String::new(),
						for_event: to_event.id.clone(),
						tag: tag.tag.clone(),
						description: tag.description.clone(),
					})
					.collect();
				for new_event in new_events.iter_mut() {
					let new_id = generate_cuid_in_transaction()?;
					new_event.id = new_id;
				}

				diesel::insert_into(tags::table)
					.values(new_events)
					.execute(&mut *db_connection)?;

				Ok(())
			});
			if let Err(error) = tx_result {
				tide::log::error!("Database error: {}", error);
				return Err(HandleConnectionError::ConnectionClosed);
			}
		}
	}

	Ok(())
}
