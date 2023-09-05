use crate::data_sync::connection::ConnectionUpdate;
use crate::data_sync::{HandleConnectionError, SubscriptionManager};
use crate::models::{
	EntryType as EntryTypeDb, Event as EventDb, EventLogEntry as EventLogEntryDb, EventLogSection as EventLogSectionDb,
	EventLogTag, Permission, PermissionEvent, Tag as TagDb, User, VideoEditState as VideoEditStateDb,
};
use crate::schema::{
	available_entry_types_for_event, entry_types, event_editors, event_log, event_log_sections, event_log_tags, events,
	permission_events, tags, user_permissions, users,
};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use chrono::prelude::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::{EventLogEntry, EventLogSection};
use stream_log_shared::messages::event_subscription::{
	EventSubscriptionData, EventSubscriptionUpdate, NewTypingData, TypingData,
};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::tags::{Tag, TagListData};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_event(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	connection_id: &str,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	event_id: &str,
	event_permission_cache: &mut HashMap<Event, Option<Permission>>,
) -> Result<(), HandleConnectionError> {
	let mut db_connection = db_connection.lock().await;
	let mut event: Vec<EventDb> = match events::table.filter(events::id.eq(event_id)).load(&mut *db_connection) {
		Ok(ev) => ev,
		Err(error) => {
			tide::log::error!("Database error loading event: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let event = match event.pop() {
		Some(ev) => ev,
		None => {
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::NoTarget,
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let event_permissions: Vec<PermissionEvent> = match permission_events::table
		.filter(
			permission_events::event.eq(event_id).and(
				user_permissions::table
					.filter(
						user_permissions::permission_group
							.eq(permission_events::permission_group)
							.and(user_permissions::user_id.eq(&user.id)),
					)
					.count()
					.single_value()
					.gt(0),
			),
		)
		.load(&mut *db_connection)
	{
		Ok(data) => data,
		Err(error) => {
			tide::log::error!("Database error retrieving event permissions: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let mut highest_permission_level: Option<Permission> = None;
	for permission in event_permissions.iter() {
		match (permission.level, highest_permission_level) {
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

	let event_data: Event = event.clone().into();
	event_permission_cache.insert(event_data, highest_permission_level);

	let permission_level = match highest_permission_level {
		Some(level) => level,
		None => {
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::NotAllowed,
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	{
		let mut subscriptions = subscription_manager.lock().await;
		subscriptions
			.subscribe_to_event(event_id, connection_id, conn_update_tx.clone())
			.await;
	}

	let entry_types: Vec<EntryTypeDb> = match entry_types::table
		.filter(
			available_entry_types_for_event::table
				.filter(
					available_entry_types_for_event::event_id
						.eq(event_id)
						.and(available_entry_types_for_event::entry_type.eq(entry_types::id)),
				)
				.count()
				.single_value()
				.gt(0),
		)
		.load(&mut *db_connection)
	{
		Ok(types) => types,
		Err(error) => {
			tide::log::error!("Database error getting event types for an event: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_from_event(event_id, connection_id)
				.await?;
			return Ok(());
		}
	};

	let tags: Vec<TagDb> = match tags::table.load(&mut *db_connection) {
		Ok(tags) => tags,
		Err(error) => {
			tide::log::error!("Database error getting tags for an event: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_from_event(event_id, connection_id)
				.await?;
			return Ok(());
		}
	};

	let mut log_sections: Vec<EventLogSectionDb> = match event_log_sections::table
		.filter(event_log_sections::event.eq(event_id))
		.order(event_log_sections::start_time.asc())
		.load(&mut *db_connection)
	{
		Ok(sections) => sections,
		Err(error) => {
			tide::log::error!("Database error getting event log sections: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_from_event(event_id, connection_id)
				.await?;
			return Ok(());
		}
	};

	let log_entries: Vec<EventLogEntryDb> = match event_log::table
		.filter(event_log::event.eq(event_id).and(event_log::deleted_by.is_null()))
		.order((
			event_log::start_time.asc(),
			event_log::manual_sort_key.asc().nulls_last(),
			event_log::created_at.asc(),
		))
		.load(&mut *db_connection)
	{
		Ok(entries) => entries,
		Err(error) => {
			tide::log::error!("Database error getting event log entries: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_from_event(event_id, connection_id)
				.await?;
			return Ok(());
		}
	};

	let log_entry_ids: Vec<String> = log_entries.iter().map(|entry| entry.id.clone()).collect();

	let log_entry_tags: Vec<EventLogTag> = match event_log_tags::table
		.filter(event_log_tags::log_entry.eq_any(&log_entry_ids))
		.load(&mut *db_connection)
	{
		Ok(tags) => tags,
		Err(error) => {
			tide::log::error!("Database error retrieving tags for event log entries: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);

			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_from_event(event_id, connection_id)
				.await?;
			return Ok(());
		}
	};

	let tags_by_id: HashMap<String, &TagDb> = tags.iter().map(|tag| (tag.id.clone(), tag)).collect();
	let mut tags_by_log_entry: HashMap<String, Vec<Tag>> = HashMap::new();
	for log_entry_tag in log_entry_tags.iter() {
		let tag = match tags_by_id.get(&log_entry_tag.tag) {
			Some(tag) => Tag {
				id: tag.id.clone(),
				name: tag.tag.clone(),
				description: tag.description.clone(),
				playlist: tag.playlist.clone(),
			},
			None => {
				let message = FromServerMessage::SubscriptionFailure(
					SubscriptionType::EventLogData(event_id.to_string()),
					SubscriptionFailureInfo::Error(DataError::ServerError),
				);
				conn_update_tx
					.send(ConnectionUpdate::SendData(Box::new(message)))
					.await?;
				subscription_manager
					.lock()
					.await
					.unsubscribe_from_event(event_id, connection_id)
					.await?;
				return Ok(());
			}
		};
		tags_by_log_entry
			.entry(log_entry_tag.log_entry.clone())
			.or_default()
			.push(tag);
	}

	let mut editor_user_ids: Vec<String> = Vec::new();
	for log_entry in log_entries.iter() {
		if let Some(user_id) = log_entry.editor.as_ref() {
			editor_user_ids.push(user_id.clone());
		}
	}
	let editor_user_ids: Vec<String> = match event_editors::table
		.filter(event_editors::event.eq(event_id))
		.select(event_editors::editor)
		.load(&mut *db_connection)
	{
		Ok(editors) => editors,
		Err(error) => {
			tide::log::error!("Database error retrieving editors for event: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_from_event(event_id, connection_id)
				.await?;
			return Ok(());
		}
	};

	let mut editors: Vec<User> = match users::table
		.filter(users::id.eq_any(&editor_user_ids))
		.load(&mut *db_connection)
	{
		Ok(users) => users,
		Err(error) => {
			tide::log::error!("Database error getting editor user data: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::EventLogData(event_id.to_string()),
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_from_event(event_id, connection_id)
				.await?;
			return Ok(());
		}
	};

	let available_editors_list: Vec<UserData> = editors
		.iter()
		.map(|user| UserData {
			id: user.id.clone(),
			username: user.name.clone(),
			is_admin: user.is_admin,
			color: user.color(),
		})
		.collect();
	let editors: HashMap<String, User> = editors.drain(..).map(|user| (user.id.clone(), user)).collect();

	// Turn all the data we have into client-usable data
	let event = Event {
		id: event.id.clone(),
		name: event.name.clone(),
		start_time: event.start_time,
	};
	let permission_level: PermissionLevel = permission_level.into();
	let entry_types: Vec<EntryType> = entry_types.iter().map(|et| (*et).clone().into()).collect();
	let event_log_sections: Vec<EventLogSection> = log_sections
		.drain(..)
		.map(|section| EventLogSection {
			id: section.id,
			name: section.name,
			start_time: section.start_time,
		})
		.collect();
	let mut event_log_entries: Vec<EventLogEntry> = Vec::with_capacity(log_entries.len());
	for log_entry in log_entries.iter() {
		let tags = match tags_by_log_entry.remove(&log_entry.id) {
			Some(tags) => tags,
			None => Vec::new(),
		};
		let editor: Option<UserData> = match &log_entry.editor {
			Some(editor) => match editors.get(editor) {
				Some(editor) => Some(UserData {
					id: editor.id.clone(),
					username: editor.name.clone(),
					is_admin: editor.is_admin,
					color: editor.color(),
				}),
				None => {
					tide::log::error!(
						"Editor {} found for log entry {} but not in users table (database constraint violation!)",
						editor,
						log_entry.id
					);
					let message = FromServerMessage::SubscriptionFailure(
						SubscriptionType::EventLogData(event_id.to_string()),
						SubscriptionFailureInfo::Error(DataError::DatabaseError),
					);
					conn_update_tx
						.send(ConnectionUpdate::SendData(Box::new(message)))
						.await?;
					subscription_manager
						.lock()
						.await
						.unsubscribe_from_event(event_id, connection_id)
						.await?;
					return Ok(());
				}
			},
			None => None,
		};
		let send_entry = EventLogEntry {
			id: log_entry.id.clone(),
			start_time: log_entry.start_time,
			end_time: log_entry.end_time,
			entry_type: log_entry.entry_type.clone(),
			description: log_entry.description.clone(),
			media_link: log_entry.media_link.clone(),
			submitter_or_winner: log_entry.submitter_or_winner.clone(),
			tags,
			notes_to_editor: log_entry.notes_to_editor.clone(),
			editor_link: log_entry.editor_link.clone(),
			editor,
			video_link: log_entry.video_link.clone(),
			parent: log_entry.parent.clone(),
			created_at: log_entry.created_at,
			manual_sort_key: log_entry.manual_sort_key,
			video_state: log_entry.video_state.map(|state| state.into()),
			video_errors: log_entry.video_errors.clone(),
			poster_moment: log_entry.poster_moment,
			video_edit_state: log_entry.video_edit_state.into(),
			marked_incomplete: log_entry.marked_incomplete,
		};
		event_log_entries.push(send_entry);
	}

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::Event(
		event,
		permission_level,
		entry_types,
		available_editors_list,
		event_log_sections,
		event_log_entries,
	)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_event_update(
	db_connection: Arc<Mutex<PgConnection>>,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	event: &Event,
	user: &UserData,
	event_permission_cache: &HashMap<Event, Option<Permission>>,
	message: Box<EventSubscriptionUpdate>,
) -> Result<(), HandleConnectionError> {
	let Some(permission_level) = event_permission_cache.get(event) else {
		// If the user is interacting with the event, they should be subscribed. Subscribing adds the event to the
		// permission cache, so we can safely abort if they don't have a cached value.
		return Ok(());
	};

	if !permission_level.map(|level| level.can_edit()).unwrap_or_default() {
		// The user doesn't have access to do this; they should either only view the data we send them or not interact
		// with it at all. Therefore, we'll ignore their request in this case.
		return Ok(());
	}

	let event_subscription_data = match *message {
		EventSubscriptionUpdate::NewLogEntry(log_entry_data, count) => {
			let mut new_entry_messages: Vec<EventSubscriptionData> = Vec::new();
			for _ in 0..count {
				let mut log_entry_data = log_entry_data.clone();
				let new_id = cuid2::create_id();

				// Store times with minute granularity
				let mut start_time = log_entry_data.start_time;
				let mut end_time = log_entry_data.end_time;

				start_time = start_time.with_second(0).unwrap();
				start_time = start_time.with_nanosecond(0).unwrap();
				if let Some(end) = end_time {
					end_time = end.with_second(0);
					if let Some(end) = end_time {
						end_time = end.with_nanosecond(0);
					}
				}

				let create_time = Utc::now();

				let db_entry = EventLogEntryDb {
					id: new_id.clone(),
					event: event.id.clone(),
					start_time,
					end_time,
					entry_type: log_entry_data.entry_type.clone(),
					description: log_entry_data.description.clone(),
					media_link: log_entry_data.media_link.clone(),
					submitter_or_winner: log_entry_data.submitter_or_winner.clone(),
					notes_to_editor: log_entry_data.notes_to_editor.clone(),
					editor_link: None,
					editor: log_entry_data.editor.clone().map(|editor| editor.id),
					video_link: None,
					last_update_user: user.id.clone(),
					last_updated: create_time,
					parent: log_entry_data.parent.clone(),
					deleted_by: None,
					created_at: create_time,
					manual_sort_key: log_entry_data.manual_sort_key,
					video_state: None,
					video_errors: String::new(),
					poster_moment: false,
					video_edit_state: log_entry_data.video_edit_state.into(),
					marked_incomplete: log_entry_data.marked_incomplete,
				};

				let saved_tags: HashMap<String, Tag> = log_entry_data
					.tags
					.iter()
					.map(|tag| (tag.id.clone(), tag.clone()))
					.collect();

				let db_tags: Vec<EventLogTag> = saved_tags
					.values()
					.map(|tag| EventLogTag {
						tag: tag.id.clone(),
						log_entry: new_id.clone(),
					})
					.collect();
				log_entry_data.tags = saved_tags.values().cloned().collect();

				let mut db_connection = db_connection.lock().await;
				let insert_result: QueryResult<(EventLogEntryDb, Vec<TagDb>, Option<User>)> = db_connection
					.transaction(|db_connection| {
						let new_row: EventLogEntryDb = diesel::insert_into(event_log::table)
							.values(db_entry)
							.get_result(db_connection)?;
						let new_row_tags: Vec<EventLogTag> = diesel::insert_into(event_log_tags::table)
							.values(db_tags)
							.get_results(db_connection)?;
						let tag_ids: Vec<String> = new_row_tags.iter().map(|tag| tag.tag.clone()).collect();
						let tags: Vec<TagDb> = tags::table.filter(tags::id.eq_any(tag_ids)).load(db_connection)?;
						let editor: Option<User> = match new_row.editor.as_ref() {
							Some(editor) => Some(users::table.find(editor).first(db_connection)?),
							None => None,
						};
						Ok((new_row, tags, editor))
					});
				let new_log_entry = match insert_result {
					Ok((entry, entry_tags, editor)) => {
						let tags: Vec<Tag> = entry_tags
							.iter()
							.map(|tag| Tag {
								id: tag.id.clone(),
								name: tag.tag.clone(),
								description: tag.description.clone(),
								playlist: tag.playlist.clone(),
							})
							.collect();
						EventLogEntry {
							id: entry.id,
							start_time: entry.start_time,
							end_time: entry.end_time,
							entry_type: entry.entry_type,
							description: entry.description,
							media_link: entry.media_link,
							submitter_or_winner: entry.submitter_or_winner,
							tags,
							video_edit_state: entry.video_edit_state.into(),
							notes_to_editor: entry.notes_to_editor,
							editor_link: entry.editor_link,
							editor: editor.map(|user| user.into()),
							video_link: entry.video_link,
							parent: entry.parent,
							created_at: entry.created_at,
							manual_sort_key: entry.manual_sort_key,
							video_state: entry.video_state.map(|state| state.into()),
							video_errors: entry.video_errors,
							poster_moment: entry.poster_moment,
							marked_incomplete: entry.marked_incomplete,
						}
					}
					Err(error) => {
						tide::log::error!("Database error adding an event log entry: {}", error);
						return Ok(());
					}
				};

				new_entry_messages.push(EventSubscriptionData::NewLogEntry(new_log_entry, user.clone()));
			}
			new_entry_messages
		}
		EventSubscriptionUpdate::DeleteLogEntry(deleted_log_entry) => {
			// Deleting an entry requires supervisor permissions, so we'll ignore requests from non-supervisors.
			if *permission_level != Some(Permission::Supervisor) {
				return Ok(());
			}

			let mut db_connection = db_connection.lock().await;
			let delete_result: QueryResult<usize> = diesel::update(event_log::table)
				.filter(
					event_log::id
						.eq(&deleted_log_entry.id)
						.and(event_log::video_link.is_null()),
				)
				.set((
					event_log::deleted_by.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
					event_log::last_update_user.eq(&user.id),
				))
				.execute(&mut *db_connection);
			if let Err(error) = delete_result {
				tide::log::error!("Database error deleting an event log entry: {}", error);
				return Ok(());
			}

			vec![EventSubscriptionData::DeleteLogEntry(deleted_log_entry)]
		}
		EventSubscriptionUpdate::ChangeStartTime(log_entry, new_start_time) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::start_time.eq(new_start_time),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry start time: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeEndTime(log_entry, new_end_time) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| -> QueryResult<EventLogEntryDb> {
				let mut updated_entry: EventLogEntryDb = diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::end_time.eq(new_end_time),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)?;
				if updated_entry.marked_incomplete
					&& updated_entry.end_time.is_some()
					&& !updated_entry.submitter_or_winner.is_empty()
				{
					updated_entry = diesel::update(event_log::table)
						.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
						.set(event_log::marked_incomplete.eq(false))
						.get_result(db_connection)?;
				}
				Ok(updated_entry)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry end time; {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeEntryType(log_entry, new_entry_type) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::entry_type.eq(&new_entry_type),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry type: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeDescription(log_entry, new_description) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::description.eq(&new_description),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry description: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeMediaLink(log_entry, new_media_link) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::media_link.eq(&new_media_link),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry media link: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeSubmitterWinner(log_entry, new_submitter_or_winner) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| -> QueryResult<EventLogEntryDb> {
				let mut updated_entry: EventLogEntryDb = diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::submitter_or_winner.eq(&new_submitter_or_winner),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)?;
				if updated_entry.marked_incomplete
					&& updated_entry.end_time.is_some()
					&& !updated_entry.submitter_or_winner.is_empty()
				{
					updated_entry = diesel::update(event_log::table)
						.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
						.set(event_log::marked_incomplete.eq(false))
						.get_result(db_connection)?;
				}
				Ok(updated_entry)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry submitter/winner: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangePosterMoment(log_entry, poster_moment) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::poster_moment.eq(poster_moment),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry poster moment: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeTags(log_entry, new_tags) => {
			let mut db_connection = db_connection.lock().await;
			let update_result: QueryResult<EventLogEntry> = db_connection.transaction(|db_connection| {
				diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::last_updated.eq(Utc::now()),
						event_log::last_update_user.eq(&user.id),
					))
					.execute(db_connection)?;
				let new_tag_ids: HashSet<String> = new_tags.iter().map(|tag| tag.id.clone()).collect();
				diesel::delete(event_log_tags::table)
					.filter(
						event_log_tags::log_entry
							.eq(&log_entry.id)
							.and(event_log_tags::tag.ne_all(&new_tag_ids)),
					)
					.execute(db_connection)?;
				let existing_tags: Vec<String> = event_log_tags::table
					.filter(
						event_log_tags::log_entry
							.eq(&log_entry.id)
							.and(event_log_tags::tag.eq_any(&new_tag_ids)),
					)
					.select(event_log_tags::tag)
					.load(&mut *db_connection)?;
				let insert_tag_ids: Vec<EventLogTag> = new_tag_ids
					.iter()
					.filter(|id| !existing_tags.contains(*id))
					.map(|id| EventLogTag {
						tag: id.clone(),
						log_entry: log_entry.id.clone(),
					})
					.collect();
				diesel::insert_into(event_log_tags::table)
					.values(insert_tag_ids)
					.execute(db_connection)?;

				let log_entry: EventLogEntryDb = event_log::table.find(&log_entry.id).first(db_connection)?;
				let mut tags: Vec<TagDb> = tags::table
					.filter(
						event_log_tags::table
							.filter(
								event_log_tags::tag
									.eq(tags::id)
									.and(event_log_tags::log_entry.eq(&log_entry.id)),
							)
							.count()
							.single_value()
							.gt(0),
					)
					.load(db_connection)?;
				let tags: Vec<Tag> = tags.drain(..).map(|tag| tag.into()).collect();
				let editor: Option<User> = match log_entry.editor {
					Some(editor) => Some(users::table.find(editor).first(db_connection)?),
					None => None,
				};
				let editor: Option<UserData> = editor.map(|editor| editor.into());

				let log_entry = EventLogEntry {
					id: log_entry.id,
					start_time: log_entry.start_time,
					end_time: log_entry.end_time,
					entry_type: log_entry.entry_type,
					description: log_entry.description,
					media_link: log_entry.media_link,
					submitter_or_winner: log_entry.submitter_or_winner,
					tags,
					notes_to_editor: log_entry.notes_to_editor,
					editor_link: log_entry.editor_link,
					editor,
					video_link: log_entry.video_link,
					parent: log_entry.parent,
					created_at: log_entry.created_at,
					manual_sort_key: log_entry.manual_sort_key,
					video_state: log_entry.video_state.map(|state| state.into()),
					video_errors: log_entry.video_errors,
					poster_moment: log_entry.poster_moment,
					video_edit_state: log_entry.video_edit_state.into(),
					marked_incomplete: log_entry.marked_incomplete,
				};
				Ok(log_entry)
			});

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry tags: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeVideoEditState(log_entry, new_video_edit_state) => {
			let new_video_edit_state: VideoEditStateDb = new_video_edit_state.into();
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::video_edit_state.eq(new_video_edit_state),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry make video flag: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeNotesToEditor(log_entry, new_notes_to_editor) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::notes_to_editor.eq(&new_notes_to_editor),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry notes to editor: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeEditor(log_entry, new_editor) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::editor.eq(new_editor.as_ref().map(|user| &user.id)),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry editor: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeIsIncomplete(log_entry, new_is_incomplete_value) => {
			// While setting this value can be done by any editor, removing it manually more strictly requires supervisor attention.
			if !new_is_incomplete_value && *permission_level != Some(Permission::Supervisor) {
				return Ok(());
			}
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				if new_is_incomplete_value {
					diesel::update(event_log::table)
						.filter(
							event_log::id
								.eq(&log_entry.id)
								.and(event_log::deleted_by.is_null())
								.and(event_log::end_time.is_null().or(event_log::submitter_or_winner.eq(""))),
						)
						.set(event_log::marked_incomplete.eq(true))
						.get_result(db_connection)
				} else {
					diesel::update(event_log::table)
						.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
						.set(event_log::marked_incomplete.eq(new_is_incomplete_value))
						.get_result(db_connection)
				}
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating incomplete entry flag: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::ChangeManualSortKey(log_entry, manual_sort_key) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				diesel::update(event_log::table)
					.filter(event_log::id.eq(&log_entry.id).and(event_log::deleted_by.is_null()))
					.set((
						event_log::manual_sort_key.eq(manual_sort_key),
						event_log::last_update_user.eq(&user.id),
						event_log::last_updated.eq(Utc::now()),
					))
					.get_result(db_connection)
			};
			let update_result = log_entry_change(&mut db_connection, update_func);

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry manual sort key: {}", error);
					return Ok(());
				}
			};
			vec![EventSubscriptionData::UpdateLogEntry(log_entry, user.clone())]
		}
		EventSubscriptionUpdate::Typing(typing_data) => {
			let user_data = UserData {
				id: user.id.clone(),
				username: user.username.clone(),
				is_admin: user.is_admin,
				color: user.color,
			};
			let typing_data = match typing_data {
				NewTypingData::Parent(log_entry, parent_entry_id) => {
					TypingData::Parent(log_entry, parent_entry_id, user_data)
				}
				NewTypingData::StartTime(log_entry, start_time_str) => {
					TypingData::StartTime(log_entry, start_time_str, user_data)
				}
				NewTypingData::EndTime(log_entry, end_time_str) => {
					TypingData::EndTime(log_entry, end_time_str, user_data)
				}
				NewTypingData::EntryType(log_entry, type_str) => TypingData::EntryType(log_entry, type_str, user_data),
				NewTypingData::Description(log_entry, description) => {
					TypingData::Description(log_entry, description, user_data)
				}
				NewTypingData::MediaLink(log_entry, media_link) => {
					TypingData::MediaLink(log_entry, media_link, user_data)
				}
				NewTypingData::SubmitterWinner(log_entry, submitter_or_winner) => {
					TypingData::SubmitterWinner(log_entry, submitter_or_winner, user_data)
				}
				NewTypingData::NotesToEditor(log_entry, notes_to_editor) => {
					TypingData::NotesToEditor(log_entry, notes_to_editor, user_data)
				}
			};
			vec![EventSubscriptionData::Typing(typing_data)]
		}
		EventSubscriptionUpdate::NewTag(mut new_tag) => {
			if new_tag.name.is_empty() || new_tag.name.contains(',') || new_tag.description.is_empty() {
				return Ok(());
			}
			let new_id = cuid2::create_id();
			new_tag.id = new_id.clone();
			let mut db_connection = db_connection.lock().await;
			let new_tag_db = TagDb {
				id: new_id,
				tag: new_tag.name.clone(),
				description: new_tag.description.clone(),
				playlist: String::new(),
			};
			let insert_result = diesel::insert_into(tags::table)
				.values(new_tag_db)
				.execute(&mut *db_connection);
			if let Err(error) = insert_result {
				tide::log::error!("Database error adding a new tag: {}", error);
				return Ok(());
			}

			let subscription_manager = subscription_manager.lock().await;
			let message = SubscriptionData::TagListUpdate(TagListData::UpdateTag(new_tag.clone()));
			let send_result = subscription_manager.broadcast_tag_list_message(message).await;
			if let Err(error) = send_result {
				tide::log::error!("Error occurred broadcasting an event tag update: {}", error);
			}

			Vec::new()
		}
	};

	let subscription_manager = subscription_manager.lock().await;
	for subscription_data in event_subscription_data {
		let subscription_data = SubscriptionData::EventUpdate(event.clone(), Box::new(subscription_data));
		let broadcast_result = subscription_manager
			.broadcast_event_message(&event.id, subscription_data)
			.await;
		if let Err(error) = broadcast_result {
			tide::log::error!("Error occurred broadcasting an event: {}", error);
		}
	}

	Ok(())
}

fn log_entry_change(
	db_connection: &mut PgConnection,
	record_update: impl FnOnce(&mut PgConnection) -> QueryResult<EventLogEntryDb>,
) -> QueryResult<EventLogEntry> {
	db_connection.transaction(|db_connection| {
		let log_entry = record_update(db_connection)?;
		let mut tags: Vec<TagDb> = tags::table
			.filter(
				event_log_tags::table
					.filter(
						event_log_tags::tag
							.eq(tags::id)
							.and(event_log_tags::log_entry.eq(&log_entry.id)),
					)
					.count()
					.single_value()
					.gt(0),
			)
			.load(db_connection)?;
		let tags: Vec<Tag> = tags.drain(..).map(|tag| tag.into()).collect();
		let editor: Option<User> = match log_entry.editor {
			Some(user_id) => Some(users::table.find(user_id).first(db_connection)?),
			None => None,
		};
		let editor = editor.map(|editor| editor.into());

		let log_entry = EventLogEntry {
			id: log_entry.id,
			start_time: log_entry.start_time,
			end_time: log_entry.end_time,
			entry_type: log_entry.entry_type,
			description: log_entry.description,
			media_link: log_entry.media_link,
			submitter_or_winner: log_entry.submitter_or_winner,
			tags,
			notes_to_editor: log_entry.notes_to_editor,
			editor_link: log_entry.editor_link,
			editor,
			video_link: log_entry.video_link,
			parent: log_entry.parent,
			created_at: log_entry.created_at,
			manual_sort_key: log_entry.manual_sort_key,
			video_state: log_entry.video_state.map(|state| state.into()),
			video_errors: log_entry.video_errors,
			poster_moment: log_entry.poster_moment,
			video_edit_state: log_entry.video_edit_state.into(),
			marked_incomplete: log_entry.marked_incomplete,
		};
		Ok(log_entry)
	})
}
