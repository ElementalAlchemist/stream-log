// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::data_sync::connection::ConnectionUpdate;
use crate::data_sync::{HandleConnectionError, SubscriptionManager};
use crate::models::{
	AvailableEntryType, EditSource, EntryType as EntryTypeDb, Event as EventDb, EventLogEntry as EventLogEntryDb,
	EventLogEntryChanges, EventLogHistoryEntry, EventLogHistoryTag, EventLogTab as EventLogTabDb, EventLogTag,
	InfoPage as InfoPageDb, Permission, PermissionEvent, Tag as TagDb, User,
};
use crate::schema::{
	available_entry_types_for_event, entry_types, event_editors, event_log, event_log_history, event_log_history_tags,
	event_log_tabs, event_log_tags, events, info_pages, permission_events, tags, user_permissions, users,
};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use chrono::prelude::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::{EndTimeData, EventLogEntry, EventLogTab};
use stream_log_shared::messages::event_subscription::{
	EventSubscriptionData, EventSubscriptionUpdate, ModifiedEventLogEntryParts, NewTypingData, TypingData,
};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::info_pages::InfoPage;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::subscriptions::{
	InitialEventSubscriptionLoadData, InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo,
	SubscriptionType,
};
use stream_log_shared::messages::tags::{Tag, TagPlaylist};
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

	let tags: Vec<TagDb> = match tags::table
		.filter(tags::deleted.eq(false).and(tags::for_event.eq(&event.id)))
		.load(&mut *db_connection)
	{
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

	let log_tabs: Vec<EventLogTabDb> = match event_log_tabs::table
		.filter(event_log_tabs::event.eq(event_id))
		.order(event_log_tabs::start_time.asc())
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
			Some(tag) => {
				let playlist = if let (Some(id), Some(title), Some(shows_in_video_descriptions)) = (
					tag.playlist.clone(),
					tag.playlist_title.clone(),
					tag.playlist_shows_in_video_descriptions,
				) {
					Some(TagPlaylist {
						id,
						title,
						shows_in_video_descriptions,
					})
				} else {
					None
				};
				Tag {
					id: tag.id.clone(),
					name: tag.tag.clone(),
					description: tag.description.clone(),
					playlist,
				}
			}
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

	let editors: Vec<User> = match users::table
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
	let editors: HashMap<String, User> = editors.into_iter().map(|user| (user.id.clone(), user)).collect();

	let info_pages: Vec<InfoPageDb> = match info_pages::table
		.filter(info_pages::event.eq(&event.id))
		.load(&mut *db_connection)
	{
		Ok(pages) => pages,
		Err(error) => {
			tide::log::error!("Database error getting event info pages: {}", error);
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

	// Turn all the data we have into client-usable data
	let event = Event {
		id: event.id.clone(),
		name: event.name.clone(),
		start_time: event.start_time,
		editor_link_format: event.editor_link_format,
		first_tab_name: event.first_tab_name,
	};
	let permission_level: PermissionLevel = permission_level.into();
	let entry_types: Vec<EntryType> = entry_types.into_iter().map(|et| et.into()).collect();
	let tags: Vec<Tag> = tags.into_iter().map(|tag| tag.into()).collect();
	let info_pages: Vec<InfoPage> = info_pages
		.into_iter()
		.map(|page| InfoPage {
			id: page.id,
			event: event.clone(),
			title: page.title,
			contents: page.contents,
		})
		.collect();
	let event_log_tabs: Vec<EventLogTab> = log_tabs
		.into_iter()
		.map(|section| EventLogTab {
			id: section.id,
			name: section.name,
			start_time: section.start_time,
		})
		.collect();
	let mut event_log_entries: Vec<EventLogEntry> = Vec::with_capacity(log_entries.len());
	for log_entry in log_entries.iter() {
		let end_time = log_entry.end_time_data();
		let tags = tags_by_log_entry.remove(&log_entry.id).unwrap_or_default();
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
			end_time,
			entry_type: log_entry.entry_type.clone(),
			description: log_entry.description.clone(),
			media_links: log_entry.media_links.iter().filter_map(|link| link.clone()).collect(),
			submitter_or_winner: log_entry.submitter_or_winner.clone(),
			tags,
			notes_to_editor: log_entry.notes_to_editor.clone(),
			editor,
			video_link: log_entry.video_link.clone(),
			parent: log_entry.parent.clone(),
			created_at: log_entry.created_at,
			manual_sort_key: log_entry.manual_sort_key,
			video_processing_state: log_entry.video_processing_state.map(|state| state.into()),
			video_errors: log_entry.video_errors.clone(),
			poster_moment: log_entry.poster_moment,
			video_edit_state: log_entry.video_edit_state.into(),
			marked_incomplete: log_entry.marked_incomplete,
		};
		event_log_entries.push(send_entry);
	}

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::Event(Box::new(
		InitialEventSubscriptionLoadData {
			event,
			permission: permission_level,
			entry_types,
			tags,
			editors: available_editors_list,
			info_pages,
			tabs: event_log_tabs,
			entries: event_log_entries,
		},
	))));
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

				start_time = start_time.with_second(0).unwrap();
				start_time = start_time.with_nanosecond(0).unwrap();
				let (end_time, end_time_incomplete) = match log_entry_data.end_time {
					EndTimeData::Time(mut end) => {
						end = end.with_second(0).unwrap();
						end = end.with_nanosecond(0).unwrap();
						(Some(end), false)
					}
					EndTimeData::NotEntered => (None, true),
					EndTimeData::NoTime => (None, false),
				};

				let create_time = Utc::now();

				let db_entry = EventLogEntryDb {
					id: new_id.clone(),
					event: event.id.clone(),
					start_time,
					end_time,
					entry_type: log_entry_data.entry_type.clone(),
					description: log_entry_data.description.clone(),
					media_links: log_entry_data
						.media_links
						.iter()
						.map(|link| Some(link.clone()))
						.collect(),
					submitter_or_winner: log_entry_data.submitter_or_winner.clone(),
					notes_to_editor: log_entry_data.notes_to_editor.clone(),
					editor: log_entry_data.editor.clone().map(|editor| editor.id),
					video_link: None,
					parent: log_entry_data.parent.clone(),
					deleted_by: None,
					created_at: create_time,
					manual_sort_key: log_entry_data.manual_sort_key,
					video_processing_state: None,
					video_errors: String::new(),
					poster_moment: false,
					video_edit_state: log_entry_data.video_edit_state.into(),
					marked_incomplete: log_entry_data.marked_incomplete,
					end_time_incomplete,
				};

				let history_entry = EventLogHistoryEntry::new_from_event_log_entry(
					&db_entry,
					Utc::now(),
					EditSource::User(user.id.clone()),
				);

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
				let history_tags: Vec<EventLogHistoryTag> = db_tags
					.iter()
					.map(|tag| EventLogHistoryTag {
						tag: tag.tag.clone(),
						history_log_entry: history_entry.id.clone(),
					})
					.collect();

				let mut db_connection = db_connection.lock().await;
				let insert_result: QueryResult<(EventLogEntryDb, Vec<TagDb>, Option<User>)> = db_connection
					.transaction(|db_connection| {
						let matching_entry_types: Vec<AvailableEntryType> = available_entry_types_for_event::table
							.filter(
								available_entry_types_for_event::event_id
									.eq(&event.id)
									.and(available_entry_types_for_event::entry_type.eq(&db_entry.entry_type)),
							)
							.limit(1)
							.load(db_connection)?;
						if matching_entry_types.is_empty() {
							return Err(diesel::result::Error::RollbackTransaction);
						}
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
						diesel::insert_into(event_log_history::table)
							.values(history_entry)
							.execute(db_connection)?;
						diesel::insert_into(event_log_history_tags::table)
							.values(history_tags)
							.execute(db_connection)?;
						Ok((new_row, tags, editor))
					});
				let new_log_entry = match insert_result {
					Ok((entry, entry_tags, editor)) => {
						let end_time = entry.end_time_data();
						let tags: Vec<Tag> = entry_tags
							.iter()
							.map(|tag| {
								let playlist = if let (Some(id), Some(title), Some(shows_in_video_descriptions)) = (
									tag.playlist.clone(),
									tag.playlist_title.clone(),
									tag.playlist_shows_in_video_descriptions,
								) {
									Some(TagPlaylist {
										id,
										title,
										shows_in_video_descriptions,
									})
								} else {
									None
								};
								Tag {
									id: tag.id.clone(),
									name: tag.tag.clone(),
									description: tag.description.clone(),
									playlist,
								}
							})
							.collect();
						EventLogEntry {
							id: entry.id,
							start_time: entry.start_time,
							end_time,
							entry_type: entry.entry_type,
							description: entry.description,
							media_links: entry.media_links.into_iter().flatten().collect(),
							submitter_or_winner: entry.submitter_or_winner,
							tags,
							video_edit_state: entry.video_edit_state.into(),
							notes_to_editor: entry.notes_to_editor,
							editor: editor.map(|user| user.into()),
							video_link: entry.video_link,
							parent: entry.parent,
							created_at: entry.created_at,
							manual_sort_key: entry.manual_sort_key,
							video_processing_state: entry.video_processing_state.map(|state| state.into()),
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
			let delete_result: QueryResult<()> = db_connection.transaction(|db_connection| {
				let deleted_entry: EventLogEntryDb = diesel::update(event_log::table)
					.filter(
						event_log::id
							.eq(&deleted_log_entry.id)
							.and(event_log::video_link.is_null()),
					)
					.set(event_log::deleted_by.eq(&user.id))
					.get_result(db_connection)?;
				let deleted_entry_tags: Vec<EventLogTag> = event_log_tags::table
					.filter(event_log_tags::log_entry.eq(&deleted_entry.id))
					.load(db_connection)?;
				let history_entry = EventLogHistoryEntry::new_from_event_log_entry(
					&deleted_entry,
					Utc::now(),
					EditSource::User(user.id.clone()),
				);
				let history_entry_tags: Vec<EventLogHistoryTag> = deleted_entry_tags
					.into_iter()
					.map(|tag| EventLogHistoryTag {
						tag: tag.tag,
						history_log_entry: history_entry.id.clone(),
					})
					.collect();
				diesel::insert_into(event_log_history::table)
					.values(history_entry)
					.execute(db_connection)?;
				diesel::insert_into(event_log_history_tags::table)
					.values(history_entry_tags)
					.execute(db_connection)?;
				Ok(())
			});
			if let Err(error) = delete_result {
				tide::log::error!("Database error deleting an event log entry: {}", error);
				return Ok(());
			}

			vec![EventSubscriptionData::DeleteLogEntry(deleted_log_entry)]
		}
		EventSubscriptionUpdate::UpdateLogEntry(log_entry, modified_parts) => {
			let mut db_connection = db_connection.lock().await;
			let update_func = |db_connection: &mut PgConnection| {
				let mut changes = EventLogEntryChanges::default();
				for part in modified_parts.iter() {
					match part {
						ModifiedEventLogEntryParts::StartTime => changes.start_time = Some(log_entry.start_time),
						ModifiedEventLogEntryParts::EndTime => match log_entry.end_time {
							EndTimeData::Time(time) => {
								changes.end_time = Some(Some(time));
								changes.end_time_incomplete = Some(false);
							}
							EndTimeData::NotEntered => {
								changes.end_time = Some(None);
								changes.end_time_incomplete = Some(true);
							}
							EndTimeData::NoTime => {
								changes.end_time = Some(None);
								changes.end_time_incomplete = Some(false);
							}
						},
						ModifiedEventLogEntryParts::EntryType => {
							changes.entry_type = Some(log_entry.entry_type.clone())
						}
						ModifiedEventLogEntryParts::Description => {
							changes.description = Some(log_entry.description.clone())
						}
						ModifiedEventLogEntryParts::MediaLinks => {
							changes.media_links =
								Some(log_entry.media_links.iter().map(|link| Some(link.clone())).collect())
						}
						ModifiedEventLogEntryParts::SubmitterOrWinner => {
							changes.submitter_or_winner = Some(log_entry.submitter_or_winner.clone())
						}
						ModifiedEventLogEntryParts::Tags => {
							let updated_tags: Vec<EventLogTag> = log_entry
								.tags
								.iter()
								.map(|tag| EventLogTag {
									tag: tag.id.clone(),
									log_entry: log_entry.id.clone(),
								})
								.collect();
							diesel::delete(event_log_tags::table)
								.filter(event_log_tags::log_entry.eq(&log_entry.id))
								.execute(db_connection)?;
							diesel::insert_into(event_log_tags::table)
								.values(updated_tags)
								.execute(db_connection)?;
						}
						ModifiedEventLogEntryParts::VideoEditState => {
							changes.video_edit_state = Some(log_entry.video_edit_state.into())
						}
						ModifiedEventLogEntryParts::PosterMoment => {
							changes.poster_moment = Some(log_entry.poster_moment)
						}
						ModifiedEventLogEntryParts::NotesToEditor => {
							changes.notes_to_editor = Some(log_entry.notes_to_editor.clone())
						}
						ModifiedEventLogEntryParts::Editor => {
							changes.editor = Some(log_entry.editor.as_ref().map(|user| user.id.clone()))
						}
						ModifiedEventLogEntryParts::MarkedIncomplete => {
							changes.marked_incomplete = Some(log_entry.marked_incomplete)
						}
						ModifiedEventLogEntryParts::SortKey => {
							changes.manual_sort_key = Some(log_entry.manual_sort_key)
						}
						ModifiedEventLogEntryParts::Parent => changes.parent = Some(log_entry.parent.clone()),
					}
				}

				if changes.has_changes() {
					diesel::update(event_log::table)
						.filter(event_log::id.eq(&log_entry.id))
						.set(changes)
						.get_result(db_connection)
				} else {
					event_log::table.find(&log_entry.id).first(db_connection)
				}
			};
			let update_result = log_entry_change(&mut db_connection, update_func, user.id.clone());

			let log_entry = match update_result {
				Ok(entry) => entry,
				Err(error) => {
					tide::log::error!("Database error updating log entry: {}", error);
					return Ok(());
				}
			};

			vec![EventSubscriptionData::UpdateLogEntry(log_entry, Some(user.clone()))]
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
				NewTypingData::MediaLinks(log_entry, media_links) => {
					TypingData::MediaLinks(log_entry, media_links, user_data)
				}
				NewTypingData::SubmitterWinner(log_entry, submitter_or_winner) => {
					TypingData::SubmitterWinner(log_entry, submitter_or_winner, user_data)
				}
				NewTypingData::NotesToEditor(log_entry, notes_to_editor) => {
					TypingData::NotesToEditor(log_entry, notes_to_editor, user_data)
				}
				NewTypingData::Clear(log_entry) => TypingData::Clear(log_entry, user_data),
			};
			vec![EventSubscriptionData::Typing(typing_data)]
		}
		EventSubscriptionUpdate::UpdateTag(mut tag) => {
			let new_tag = tag.id.is_empty();
			if new_tag {
				if tag.name.is_empty() || tag.name.contains(',') || tag.description.is_empty() {
					return Ok(());
				}
				tag.id = cuid2::create_id();
			}
			let (playlist, playlist_title, playlist_shows_in_video_descriptions) =
				if let Some(playlist) = tag.playlist.as_ref() {
					(
						Some(playlist.id.clone()),
						Some(playlist.title.clone()),
						Some(playlist.shows_in_video_descriptions),
					)
				} else {
					(None, None, None)
				};

			let mut db_connection = db_connection.lock().await;
			let tag_db = TagDb {
				id: tag.id.clone(),
				tag: tag.name.clone(),
				description: tag.description.clone(),
				for_event: event.id.clone(),
				deleted: false,
				playlist,
				playlist_title,
				playlist_shows_in_video_descriptions,
			};
			let db_result: QueryResult<bool> = db_connection.transaction(|db_connection| {
				if new_tag {
					diesel::insert_into(tags::table)
						.values(&tag_db)
						.execute(db_connection)?;
					Ok(true)
				} else {
					let this_tag: TagDb = tags::table.find(&tag_db.id).first(db_connection)?;
					if this_tag.for_event != event.id {
						return Ok(false);
					}
					diesel::update(tags::table)
						.filter(tags::id.eq(&tag_db.id))
						.set(&tag_db)
						.execute(db_connection)?;
					Ok(true)
				}
			});
			match db_result {
				Ok(true) => (),
				Ok(false) => return Ok(()),
				Err(error) => {
					tide::log::error!("Database error updating a tag: {}", error);
					return Ok(());
				}
			}

			vec![EventSubscriptionData::UpdateTag(tag)]
		}
		EventSubscriptionUpdate::RemoveTag(tag) => {
			if *permission_level != Some(Permission::Supervisor) {
				return Ok(());
			}
			let mut db_connection = db_connection.lock().await;
			let delete_result: QueryResult<bool> = db_connection.transaction(|db_connection| {
				let this_tag: TagDb = tags::table.find(&tag.id).first(db_connection)?;
				if this_tag.for_event != event.id {
					return Ok(false);
				}
				diesel::update(tags::table)
					.filter(tags::id.eq(&tag.id))
					.set(tags::deleted.eq(true))
					.execute(db_connection)?;
				Ok(true)
			});
			match delete_result {
				Ok(true) => (),
				Ok(false) => return Ok(()),
				Err(error) => {
					tide::log::error!("Database error removing a tag: {}", error);
					return Ok(());
				}
			}

			vec![EventSubscriptionData::RemoveTag(tag)]
		}
		EventSubscriptionUpdate::ReplaceTag(tag, replacement_tag) => {
			if *permission_level != Some(Permission::Supervisor) {
				return Ok(());
			}
			let mut db_connection = db_connection.lock().await;
			let replace_result: QueryResult<(bool, Vec<EventLogEntry>)> = db_connection.transaction(|db_connection| {
				let original_tag: TagDb = tags::table.find(&tag.id).first(db_connection)?;
				let replacement: TagDb = tags::table.find(&replacement_tag.id).first(db_connection)?;
				if original_tag.for_event != event.id || replacement.for_event != event.id {
					return Ok((false, Vec::new()));
				}

				let log_entry_tags: Vec<EventLogTag> = event_log_tags::table
					.filter(event_log_tags::tag.eq(&tag.id))
					.load(db_connection)?;
				let entry_tags: Vec<EventLogTag> = log_entry_tags
					.iter()
					.map(|log_entry_tag| EventLogTag {
						tag: replacement_tag.id.clone(),
						log_entry: log_entry_tag.log_entry.clone(),
					})
					.collect();
				diesel::insert_into(event_log_tags::table)
					.values(&entry_tags)
					.on_conflict_do_nothing()
					.execute(db_connection)?;
				diesel::delete(event_log_tags::table)
					.filter(event_log_tags::tag.eq(&tag.id))
					.execute(db_connection)?;
				diesel::update(tags::table)
					.filter(tags::id.eq(&tag.id))
					.set(tags::deleted.eq(true))
					.execute(db_connection)?;

				let log_entry_ids: Vec<String> = log_entry_tags
					.iter()
					.map(|tag_entry| tag_entry.log_entry.clone())
					.collect();
				let affected_log_entries: Vec<EventLogEntryDb> = event_log::table
					.filter(event_log::id.eq_any(log_entry_ids))
					.load(db_connection)?;

				let mut output_log_entries: Vec<EventLogEntry> = Vec::with_capacity(affected_log_entries.len());
				for log_entry in affected_log_entries.iter() {
					let end_time = log_entry.end_time_data();

					let tag_ids: Vec<String> = entry_tags
						.iter()
						.filter(|entry_tag| entry_tag.log_entry == log_entry.id)
						.map(|entry_tag| entry_tag.tag.clone())
						.collect();
					let tags: Vec<TagDb> = tags::table.filter(tags::id.eq_any(tag_ids)).load(db_connection)?;
					let tags: Vec<Tag> = tags.into_iter().map(|tag| tag.into()).collect();

					let editor = match log_entry.editor.as_ref() {
						Some(editor) => {
							let editor: User = users::table.find(editor).first(db_connection)?;
							let editor: UserData = editor.into();
							Some(editor)
						}
						None => None,
					};

					let updated_entry = EventLogEntry {
						id: log_entry.id.clone(),
						start_time: log_entry.start_time,
						end_time,
						entry_type: log_entry.entry_type.clone(),
						description: log_entry.description.clone(),
						media_links: log_entry.media_links.iter().filter_map(|link| link.clone()).collect(),
						submitter_or_winner: log_entry.submitter_or_winner.clone(),
						tags,
						notes_to_editor: log_entry.notes_to_editor.clone(),
						editor,
						video_link: log_entry.video_link.clone(),
						parent: log_entry.parent.clone(),
						created_at: log_entry.created_at,
						manual_sort_key: log_entry.manual_sort_key,
						video_processing_state: log_entry
							.video_processing_state
							.map(|video_processing_state| video_processing_state.into()),
						video_errors: log_entry.video_errors.clone(),
						poster_moment: log_entry.poster_moment,
						video_edit_state: log_entry.video_edit_state.into(),
						marked_incomplete: log_entry.marked_incomplete,
					};
					output_log_entries.push(updated_entry);
				}

				Ok((true, output_log_entries))
			});
			let log_entries = match replace_result {
				Ok((true, entries)) => entries,
				Ok((false, _)) => return Ok(()),
				Err(error) => {
					tide::log::error!("Database error replacing a tag: {}", error);
					return Ok(());
				}
			};
			let mut send_messages: Vec<EventSubscriptionData> = Vec::with_capacity(log_entries.len() + 1);
			for log_entry in log_entries.into_iter() {
				send_messages.push(EventSubscriptionData::UpdateLogEntry(log_entry, Some(user.clone())));
			}
			send_messages.push(EventSubscriptionData::RemoveTag(tag));

			send_messages
		}
		EventSubscriptionUpdate::CopyTagsFromEvent(copy_from_event) => {
			if !user.is_admin {
				return Ok(());
			}

			let mut db_connection = db_connection.lock().await;
			let added_tags: QueryResult<Vec<TagDb>> = db_connection.transaction(|db_connection| {
				let event_tags: Vec<TagDb> = tags::table
					.filter(tags::for_event.eq(copy_from_event.id).and(tags::deleted.eq(false)))
					.load(db_connection)?;
				let event_tag_names: Vec<String> = event_tags.iter().map(|tag| tag.tag.clone()).collect();
				let overlapping_event_tag_names: Vec<String> = tags::table
					.filter(
						tags::for_event
							.eq(&event.id)
							.and(tags::tag.eq_any(&event_tag_names))
							.and(tags::deleted.eq(false)),
					)
					.select(tags::tag)
					.load(db_connection)?;
				let overlapping_event_tag_names: HashSet<String> = overlapping_event_tag_names.into_iter().collect();
				let new_event_tags: Vec<TagDb> = event_tags
					.iter()
					.filter(|tag| !overlapping_event_tag_names.contains(&tag.tag))
					.map(|tag| TagDb {
						id: cuid2::create_id(),
						tag: tag.tag.clone(),
						description: tag.description.clone(),
						for_event: event.id.clone(),
						deleted: false,
						playlist: None,
						playlist_title: None,
						playlist_shows_in_video_descriptions: None,
					})
					.collect();
				diesel::insert_into(tags::table)
					.values(&new_event_tags)
					.execute(db_connection)?;

				Ok(new_event_tags)
			});
			let added_tags: Vec<Tag> = match added_tags {
				Ok(tags) => tags.into_iter().map(|tag| tag.into()).collect(),
				Err(error) => {
					tide::log::error!("Database error copying event tags: {}", error);
					return Ok(());
				}
			};

			added_tags.into_iter().map(EventSubscriptionData::UpdateTag).collect()
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
	update_user_id: String,
) -> QueryResult<EventLogEntry> {
	db_connection.transaction(|db_connection| {
		let log_entry = record_update(db_connection)?;

		let end_time = log_entry.end_time_data();

		let tags: Vec<TagDb> = tags::table
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

		let history_entry =
			EventLogHistoryEntry::new_from_event_log_entry(&log_entry, Utc::now(), EditSource::User(update_user_id));
		let history_entry_tags: Vec<EventLogHistoryTag> = tags
			.iter()
			.map(|tag| EventLogHistoryTag {
				tag: tag.id.clone(),
				history_log_entry: history_entry.id.clone(),
			})
			.collect();
		diesel::insert_into(event_log_history::table)
			.values(history_entry)
			.execute(db_connection)?;
		diesel::insert_into(event_log_history_tags::table)
			.values(history_entry_tags)
			.execute(db_connection)?;

		let tags: Vec<Tag> = tags.into_iter().map(|tag| tag.into()).collect();
		let editor: Option<User> = match log_entry.editor {
			Some(user_id) => Some(users::table.find(user_id).first(db_connection)?),
			None => None,
		};
		let editor = editor.map(|editor| editor.into());

		let log_entry = EventLogEntry {
			id: log_entry.id,
			start_time: log_entry.start_time,
			end_time,
			entry_type: log_entry.entry_type,
			description: log_entry.description,
			media_links: log_entry.media_links.into_iter().flatten().collect(),
			submitter_or_winner: log_entry.submitter_or_winner,
			tags,
			notes_to_editor: log_entry.notes_to_editor,
			editor,
			video_link: log_entry.video_link,
			parent: log_entry.parent,
			created_at: log_entry.created_at,
			manual_sort_key: log_entry.manual_sort_key,
			video_processing_state: log_entry.video_processing_state.map(|state| state.into()),
			video_errors: log_entry.video_errors,
			poster_moment: log_entry.poster_moment,
			video_edit_state: log_entry.video_edit_state.into(),
			marked_incomplete: log_entry.marked_incomplete,
		};
		Ok(log_entry)
	})
}
