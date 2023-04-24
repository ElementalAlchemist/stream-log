use crate::data_sync::connection::ConnectionUpdate;
use crate::data_sync::{HandleConnectionError, SubscriptionManager};
use crate::models::{
	EntryType as EntryTypeDb, Event as EventDb, EventLogEntry as EventLogEntryDb, EventLogTag, Permission,
	PermissionEvent, Tag as TagDb, User,
};
use crate::schema::{
	available_entry_types_for_event, entry_types, event_editors, event_log, event_log_tags, events, permission_events,
	tags, user_permissions, users,
};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use chrono::Utc;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::event_subscription::{
	EventSubscriptionData, EventSubscriptionUpdate, NewTypingData, TypingData,
};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_event(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
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
		match permission.level {
			Permission::Edit => {
				highest_permission_level = Some(Permission::Edit);
				break;
			}
			Permission::View => highest_permission_level = Some(Permission::View),
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
			.subscribe_user_to_event(event_id, user, conn_update_tx.clone())
			.await;
	}

	let event_types: Vec<EntryTypeDb> = match entry_types::table
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
				.unsubscribe_user_from_event(event_id, user)
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
				.unsubscribe_user_from_event(event_id, user)
				.await?;
			return Ok(());
		}
	};

	let log_entries: Vec<EventLogEntryDb> = match event_log::table
		.filter(event_log::event.eq(event_id))
		.order(event_log::start_time.asc())
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
				.unsubscribe_user_from_event(event_id, user)
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
				.unsubscribe_user_from_event(event_id, user)
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
					.unsubscribe_user_from_event(event_id, user)
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
				.unsubscribe_user_from_event(event_id, user)
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
				.unsubscribe_user_from_event(event_id, user)
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
	let entry_types: Vec<EntryType> = event_types
		.iter()
		.map(|et| EntryType {
			id: et.id.clone(),
			name: et.name.clone(),
			color: et.color(),
		})
		.collect();
	let tags: Vec<Tag> = tags
		.iter()
		.map(|t| Tag {
			id: t.id.clone(),
			name: t.tag.clone(),
			description: t.description.clone(),
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
						.unsubscribe_user_from_event(event_id, user)
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
			make_video: log_entry.make_video,
			notes_to_editor: log_entry.notes_to_editor.clone(),
			editor_link: log_entry.editor_link.clone(),
			editor,
			video_link: log_entry.video_link.clone(),
			highlighted: log_entry.highlighted,
			parent: None,
		};
		event_log_entries.push(send_entry);
	}

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::Event(
		event,
		permission_level,
		entry_types,
		tags,
		available_editors_list,
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

	if *permission_level != Some(Permission::Edit) {
		// The user doesn't have access to do this; they should either only view the data we send them or not interact
		// with it at all. Therefore, we'll ignore their request in this case.
		return Ok(());
	}

	let subscription_manager = subscription_manager.lock().await;

	let subscription_data = match *message {
		EventSubscriptionUpdate::NewLogEntry(mut log_entry_data) => {
			let new_id = cuid2::create_id();
			let db_entry = EventLogEntryDb {
				id: new_id.clone(),
				event: event.id.clone(),
				start_time: log_entry_data.start_time,
				end_time: log_entry_data.end_time,
				entry_type: log_entry_data.entry_type.clone(),
				description: log_entry_data.description.clone(),
				media_link: log_entry_data.media_link.clone(),
				submitter_or_winner: log_entry_data.submitter_or_winner.clone(),
				make_video: log_entry_data.make_video,
				notes_to_editor: log_entry_data.notes_to_editor.clone(),
				editor_link: None,
				editor: log_entry_data.editor.clone().map(|editor| editor.id),
				video_link: None,
				highlighted: log_entry_data.highlighted,
				last_update_user: user.id.clone(),
				last_updated: Utc::now(),
				parent: None,
			};

			let db_tags: Vec<EventLogTag> = log_entry_data
				.tags
				.iter()
				.map(|tag| EventLogTag {
					tag: tag.id.clone(),
					log_entry: log_entry_data.id.clone(),
				})
				.collect();

			let mut db_connection = db_connection.lock().await;
			let insert_result: QueryResult<()> = db_connection.transaction(|db_connection| {
				diesel::insert_into(event_log::table)
					.values(db_entry)
					.execute(db_connection)?;
				diesel::insert_into(event_log_tags::table)
					.values(db_tags)
					.execute(db_connection)?;
				Ok(())
			});
			if let Err(error) = insert_result {
				tide::log::error!("Database error adding an event log entry: {}", error);
				return Ok(());
			}

			log_entry_data.id = new_id;
			EventSubscriptionData::NewLogEntry(log_entry_data)
		}
		EventSubscriptionUpdate::DeleteLogEntry(deleted_log_entry) => {
			let mut db_connection = db_connection.lock().await;
			let delete_result: QueryResult<()> = db_connection.transaction(|db_connection| {
				diesel::delete(event_log_tags::table)
					.filter(event_log_tags::log_entry.eq(&deleted_log_entry.id))
					.execute(db_connection)?;
				diesel::delete(event_log::table)
					.filter(event_log::id.eq(&deleted_log_entry.id))
					.execute(db_connection)?;
				Ok(())
			});
			if let Err(error) = delete_result {
				tide::log::error!("Database error deleting an event log entry: {}", error);
				return Ok(());
			}

			EventSubscriptionData::DeleteLogEntry(deleted_log_entry)
		}
		EventSubscriptionUpdate::ChangeStartTime(mut log_entry, new_start_time) => {
			let mut db_connection = db_connection.lock().await;
			let update_result = diesel::update(event_log::table)
				.filter(event_log::id.eq(&log_entry.id))
				.set((
					event_log::start_time.eq(new_start_time),
					event_log::last_update_user.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry start time: {}", error);
				return Ok(());
			}

			log_entry.start_time = new_start_time;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::ChangeEndTime(mut log_entry, new_end_time) => {
			let mut db_connection = db_connection.lock().await;
			let update_result = diesel::update(event_log::table)
				.filter(event_log::id.eq(&log_entry.id))
				.set((
					event_log::end_time.eq(new_end_time),
					event_log::last_update_user.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry end time; {}", error);
				return Ok(());
			}

			log_entry.end_time = new_end_time;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::ChangeEntryType(mut log_entry, new_entry_type) => {
			let mut db_connection = db_connection.lock().await;
			let update_result = diesel::update(event_log::table)
				.filter(event_log::id.eq(&log_entry.id))
				.set((
					event_log::entry_type.eq(&new_entry_type),
					event_log::last_update_user.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry type: {}", error);
				return Ok(());
			}

			log_entry.entry_type = new_entry_type;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::ChangeDescription(mut log_entry, new_description) => {
			let mut db_connection = db_connection.lock().await;
			let update_result = diesel::update(event_log::table)
				.filter(event_log::id.eq(&log_entry.id))
				.set((
					event_log::description.eq(&new_description),
					event_log::last_update_user.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry description: {}", error);
				return Ok(());
			}

			log_entry.description = new_description;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::ChangeMediaLink(mut log_entry, new_media_link) => {
			let mut db_connection = db_connection.lock().await;
			let update_result = diesel::update(event_log::table)
				.filter(event_log::id.eq(&log_entry.id))
				.set((
					event_log::media_link.eq(&new_media_link),
					event_log::last_update_user.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry media link: {}", error);
				return Ok(());
			}

			log_entry.media_link = new_media_link;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::ChangeSubmitterWinner(mut log_entry, new_submitter_or_winner) => {
			let mut db_connection = db_connection.lock().await;
			let update_result = diesel::update(event_log::table)
				.filter(event_log::id.eq(&log_entry.id))
				.set((
					event_log::submitter_or_winner.eq(&new_submitter_or_winner),
					event_log::last_update_user.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry submitter/winner: {}", error);
				return Ok(());
			}

			log_entry.submitter_or_winner = new_submitter_or_winner;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::ChangeTags(mut log_entry, new_tags) => {
			let mut db_connection = db_connection.lock().await;
			let update_result: QueryResult<()> = db_connection.transaction(|db_connection| {
				let new_tag_ids: Vec<String> = new_tags.iter().map(|tag| tag.id.clone()).collect();
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
				Ok(())
			});
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry tags: {}", error);
				return Ok(());
			}

			log_entry.tags = new_tags;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::ChangeMakeVideo(mut log_entry, new_make_video_value) => {
			let mut db_connection = db_connection.lock().await;
			let update_result = diesel::update(event_log::table)
				.filter(event_log::id.eq(&log_entry.id))
				.set((
					event_log::make_video.eq(new_make_video_value),
					event_log::last_update_user.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry make video flag: {}", error);
				return Ok(());
			}

			log_entry.make_video = new_make_video_value;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::ChangeNotesToEditor(mut log_entry, new_notes_to_editor) => {
			let mut db_connection = db_connection.lock().await;
			let update_result = diesel::update(event_log::table)
				.filter(event_log::id.eq(&user.id))
				.set((
					event_log::notes_to_editor.eq(&new_notes_to_editor),
					event_log::last_update_user.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry notes to editor: {}", error);
				return Ok(());
			}

			log_entry.notes_to_editor = new_notes_to_editor;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::ChangeEditor(mut log_entry, new_editor) => {
			let mut db_connection = db_connection.lock().await;
			let update_result = diesel::update(event_log::table)
				.filter(event_log::id.eq(&log_entry.id))
				.set((
					event_log::editor.eq(new_editor.as_ref().map(|user| &user.id)),
					event_log::last_update_user.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry editor: {}", error);
				return Ok(());
			}

			log_entry.editor = new_editor;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::ChangeHighlighted(mut log_entry, new_highlighted_value) => {
			let mut db_connection = db_connection.lock().await;
			let update_result = diesel::update(event_log::table)
				.filter(event_log::id.eq(&log_entry.id))
				.set((
					event_log::highlighted.eq(new_highlighted_value),
					event_log::last_update_user.eq(&user.id),
					event_log::last_updated.eq(Utc::now()),
				))
				.execute(&mut *db_connection);
			if let Err(error) = update_result {
				tide::log::error!("Database error updating log entry highlight: {}", error);
				return Ok(());
			}

			log_entry.highlighted = new_highlighted_value;
			EventSubscriptionData::UpdateLogEntry(log_entry)
		}
		EventSubscriptionUpdate::Typing(typing_data) => {
			let user_data = UserData {
				id: user.id.clone(),
				username: user.username.clone(),
				is_admin: user.is_admin,
				color: user.color,
			};
			let typing_data = match typing_data {
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
			EventSubscriptionData::Typing(event.clone(), typing_data)
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
			};
			let insert_result = diesel::insert_into(tags::table)
				.values(new_tag_db)
				.execute(&mut *db_connection);
			if let Err(error) = insert_result {
				tide::log::error!("Database error adding a new tag: {}", error);
				return Ok(());
			}
			EventSubscriptionData::NewTag(event.clone(), new_tag)
		}
	};
	let subscription_data = SubscriptionData::EventUpdate(event.clone(), Box::new(subscription_data));
	let broadcast_result = subscription_manager
		.broadcast_event_message(&event.id, subscription_data)
		.await;
	match broadcast_result {
		Ok(_) => Ok(()),
		Err(error) => {
			tide::log::error!("Error occurred broadcasting an event: {}", error);
			Err(HandleConnectionError::ConnectionClosed)
		}
	}
}

pub async fn unsubscribe_from_event(
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	user: &UserData,
	event_id: &str,
) -> tide::Result<()> {
	let subscription_manager = subscription_manager.lock().await;
	subscription_manager.unsubscribe_user_from_event(event_id, user).await
}