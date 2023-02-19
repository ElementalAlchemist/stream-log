use super::HandleConnectionError;
use crate::models::{
	EntryType as EntryTypeDb, Event as EventDb, EventLogEntry as EventLogEntryDb, EventLogTag, Permission,
	PermissionEvent, Tag as TagDb, User,
};
use crate::schema::{
	available_entry_types_for_event, entry_types, event_log, event_log_tags, events, permission_events, tags,
	user_permissions, users,
};
use crate::synchronization::SubscriptionManager;
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::event_subscription::EventSubscriptionResponse;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::DataError;
use tide_websockets::WebSocketConnection;

pub async fn subscribe_to_event(
	db_connection: Arc<Mutex<PgConnection>>,
	stream: Arc<Mutex<WebSocketConnection>>,
	user: &User,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	event_id: &str,
) -> Result<(), HandleConnectionError> {
	let mut db_connection = db_connection.lock().await;
	let mut event: Vec<EventDb> = match events::table.filter(events::id.eq(event_id)).load(&mut *db_connection) {
		Ok(ev) => ev,
		Err(error) => {
			tide::log::error!("Database error loading event: {}", error);
			let message = EventSubscriptionResponse::Error(DataError::DatabaseError);
			stream.lock().await.send_json(&message).await?;
			return Ok(());
		}
	};

	let event = match event.pop() {
		Some(ev) => ev,
		None => {
			let message = EventSubscriptionResponse::NoEvent;
			stream.lock().await.send_json(&message).await?;
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
			let message = EventSubscriptionResponse::Error(DataError::DatabaseError);
			stream.lock().await.send_json(&message).await?;
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

	let permission_level = match highest_permission_level {
		Some(level) => level,
		None => {
			let message = EventSubscriptionResponse::NotAllowed;
			stream.lock().await.send_json(&message).await?;
			return Ok(());
		}
	};

	// We lock the stream before adding it as a subscription to ensure the initial data sync occurs before subscription messages start flowing in
	let send_stream = stream.lock().await;
	{
		let mut subscriptions = subscription_manager.lock().await;
		subscriptions.subscribe_user_to_event(event_id.to_owned(), user, Arc::clone(&stream));
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
			let message = EventSubscriptionResponse::Error(DataError::DatabaseError);
			send_stream.send_json(&message).await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_user_from_event(event_id, user);
			return Ok(());
		}
	};

	let tags: Vec<TagDb> = match tags::table
		.filter(tags::for_event.eq(event_id))
		.load(&mut *db_connection)
	{
		Ok(tags) => tags,
		Err(error) => {
			tide::log::error!("Database error getting tags for an event: {}", error);
			let message = EventSubscriptionResponse::Error(DataError::DatabaseError);
			send_stream.send_json(&message).await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_user_from_event(event_id, user);
			return Ok(());
		}
	};

	let log_entries: Vec<EventLogEntryDb> = match event_log::table
		.filter(event_log::event.eq(event_id))
		.load(&mut *db_connection)
	{
		Ok(entries) => entries,
		Err(error) => {
			tide::log::error!("Database error getting event log entries: {}", error);
			let message = EventSubscriptionResponse::Error(DataError::DatabaseError);
			send_stream.send_json(&message).await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_user_from_event(event_id, user);
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
			let message = EventSubscriptionResponse::Error(DataError::DatabaseError);
			send_stream.send_json(&message).await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_user_from_event(event_id, user);
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
				let message = EventSubscriptionResponse::Error(DataError::ServerError);
				send_stream.send_json(&message).await?;
				subscription_manager
					.lock()
					.await
					.unsubscribe_user_from_event(event_id, user);
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

	let mut editors: Vec<User> = match users::table
		.filter(users::id.eq_any(&editor_user_ids))
		.load(&mut *db_connection)
	{
		Ok(users) => users,
		Err(error) => {
			tide::log::error!("Database error getting editor user data: {}", error);
			let message = EventSubscriptionResponse::Error(DataError::DatabaseError);
			send_stream.send_json(&message).await?;
			subscription_manager
				.lock()
				.await
				.unsubscribe_user_from_event(event_id, user);
			return Ok(());
		}
	};

	let editors: HashMap<String, User> = editors.drain(..).map(|user| (user.id.clone(), user)).collect();

	// Turn all the data we have into client-usable data
	let event = Event {
		id: event.id.clone(),
		name: event.name.clone(),
		start_time: event.start_time,
	};
	let permission_level: PermissionLevel = permission_level.into();
	let event_types: Vec<EntryType> = event_types
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
					let message = EventSubscriptionResponse::Error(DataError::DatabaseError);
					send_stream.send_json(&message).await?;
					subscription_manager
						.lock()
						.await
						.unsubscribe_user_from_event(event_id, user);
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
		};
		event_log_entries.push(send_entry);
	}

	let message = EventSubscriptionResponse::Subscribed(event, permission_level, event_types, tags, event_log_entries);
	send_stream.send_json(&message).await?;

	Ok(())
}

pub async fn unsubscribe_all(stream: Arc<Mutex<WebSocketConnection>>, user: &User) {
	// TODO
}
