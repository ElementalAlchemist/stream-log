use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::{Event as EventDb, EventLogEntry as EventLogEntryDb, EventLogTag, Tag as TagDb, User};
use crate::schema::{event_log, event_log_tags, events, tags, users};
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use stream_log_shared::messages::admin::{AdminTagData, AdminTagUpdate};
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::tags::{AvailableTagData, Tag};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_tags(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message =
			FromServerMessage::SubscriptionFailure(SubscriptionType::AdminTags, SubscriptionFailureInfo::NotAllowed);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = db_connection.lock().await;
	let tags: QueryResult<Vec<TagDb>> = tags::table.load(&mut *db_connection);
	let tags: Vec<Tag> = match tags {
		Ok(mut tags) => tags.drain(..).map(|tag| tag.into()).collect(),
		Err(error) => {
			tide::log::error!(
				"A database error occurred retrieving tag data for admin subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminTags,
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
		.add_admin_tags_subscription(user, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminTags(tags)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}

pub async fn handle_admin_tags_message(
	db_connection: Arc<Mutex<PgConnection>>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	update_message: AdminTagUpdate,
) {
	if !user.is_admin {
		return;
	}
	if !subscription_manager
		.lock()
		.await
		.user_is_subscribed_to_admin_tags(user)
		.await
	{
		return;
	}

	match update_message {
		AdminTagUpdate::UpdateTag(mut tag) => {
			{
				if tag.id.is_empty() {
					tag.id = cuid2::create_id();
				}
				let tag_db = TagDb {
					id: tag.id.clone(),
					tag: tag.name.clone(),
					description: tag.description.clone(),
				};
				let mut db_connection = db_connection.lock().await;
				let db_result = diesel::insert_into(tags::table)
					.values(tag_db)
					.on_conflict(tags::id)
					.do_update()
					.set((tags::tag.eq(&tag.name), tags::description.eq(&tag.description)))
					.execute(&mut *db_connection);
				if let Err(error) = db_result {
					tide::log::error!("A database error occurred updating a tag: {}", error);
					return;
				}
			}

			let subscription_manager = subscription_manager.lock().await;
			let message = SubscriptionData::AdminTagsUpdate(AdminTagData::UpdateTag(tag.clone()));
			let send_result = subscription_manager.broadcast_admin_tags_message(message).await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send admin tags update message: {}", error);
			}
			let message = SubscriptionData::AvailableTagsUpdate(AvailableTagData::UpdateTag(tag));
			let send_result = subscription_manager.broadcast_available_tags_message(message).await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send available tags update message: {}", error);
			}
		}
		AdminTagUpdate::ReplaceTag(remove_tag, replacement_tag) => {
			let event_update_data = {
				let mut db_connection = db_connection.lock().await;
				let tx_result: QueryResult<Vec<(Event, EventLogEntry)>> = db_connection.transaction(|db_connection| {
					let log_entry_tags: Vec<EventLogTag> = event_log_tags::table
						.filter(event_log_tags::tag.eq(&remove_tag.id))
						.load(db_connection)?;
					let replaced_tags: Vec<EventLogTag> = log_entry_tags
						.iter()
						.map(|entry| EventLogTag {
							tag: replacement_tag.id.clone(),
							log_entry: entry.log_entry.clone(),
						})
						.collect();
					let log_entry_ids: Vec<String> = log_entry_tags
						.iter()
						.map(|entry_tag| entry_tag.log_entry.clone())
						.collect();
					let log_entries: Vec<EventLogEntryDb> = event_log::table
						.filter(event_log::id.eq_any(&log_entry_ids))
						.load(db_connection)?;
					diesel::insert_into(event_log_tags::table)
						.values(replaced_tags)
						.on_conflict_do_nothing()
						.execute(db_connection)?;

					diesel::delete(tags::table)
						.filter(tags::id.eq(&remove_tag.id))
						.execute(db_connection)?;

					let mut all_events: Vec<EventDb> = events::table.load(db_connection)?;
					let all_events: Vec<Event> = all_events.drain(..).map(|event| event.into()).collect();

					let mut event_log_entries: Vec<(Event, EventLogEntry)> = Vec::with_capacity(log_entries.len());
					for log_entry in log_entries {
						let tags: Vec<String> = log_entry_tags
							.iter()
							.filter(|entry_tag| entry_tag.log_entry == log_entry.id)
							.map(|entry_tag| entry_tag.tag.clone())
							.collect();
						let mut tags: Vec<TagDb> = tags::table.filter(tags::id.eq_any(&tags)).load(db_connection)?;
						let tags: Vec<Tag> = tags.drain(..).map(|tag| tag.into()).collect();
						let event = all_events
							.iter()
							.find(|event| event.id == log_entry.event)
							.unwrap()
							.clone();
						let editor: Option<User> = if let Some(editor_id) = log_entry.editor.as_ref() {
							Some(users::table.find(editor_id).first(db_connection)?)
						} else {
							None
						};
						let editor: Option<UserData> = editor.map(|editor| editor.into());

						event_log_entries.push((
							event,
							EventLogEntry {
								id: log_entry.id,
								start_time: log_entry.start_time,
								end_time: log_entry.end_time,
								entry_type: log_entry.entry_type,
								description: log_entry.description,
								media_link: log_entry.media_link,
								submitter_or_winner: log_entry.submitter_or_winner,
								tags,
								make_video: log_entry.make_video,
								notes_to_editor: log_entry.notes_to_editor,
								editor_link: log_entry.editor_link,
								editor,
								video_link: log_entry.video_link,
								highlighted: log_entry.highlighted,
								parent: log_entry.parent,
								created_at: log_entry.created_at,
								manual_sort_key: log_entry.manual_sort_key,
							},
						));
					}

					Ok(event_log_entries)
				});
				match tx_result {
					Ok(data) => data,
					Err(error) => {
						tide::log::error!("A database error occurred replacing a tag: {}", error);
						return;
					}
				}
			};

			let subscription_manager = subscription_manager.lock().await;
			let admin_message = SubscriptionData::AdminTagsUpdate(AdminTagData::RemoveTag(remove_tag.clone()));
			let send_result = subscription_manager.broadcast_admin_tags_message(admin_message).await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send tag update message for tag replacement: {}", error);
			}
			let available_message =
				SubscriptionData::AvailableTagsUpdate(AvailableTagData::RemoveTag(remove_tag.clone()));
			let send_result = subscription_manager
				.broadcast_available_tags_message(available_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send tag update message for tag replacement: {}", error);
			}

			for (log_entry_event, log_entry) in event_update_data {
				let event_id = log_entry_event.id.clone();
				let event_message = SubscriptionData::EventUpdate(
					log_entry_event,
					Box::new(EventSubscriptionData::UpdateLogEntry(log_entry)),
				);
				let send_result = subscription_manager
					.broadcast_event_message(&event_id, event_message)
					.await;
				if let Err(error) = send_result {
					tide::log::error!(
						"Failed to broadcast event log entry update for tag replacement: {}",
						error
					);
				}
			}
		}
		AdminTagUpdate::RemoveTag(tag) => {
			let event_update_data = {
				let mut db_connection = db_connection.lock().await;
				let tx_result: QueryResult<Vec<(Event, EventLogEntry)>> = db_connection.transaction(|db_connection| {
					let removed_entry_tags: Vec<EventLogTag> = diesel::delete(event_log_tags::table)
						.filter(event_log_tags::tag.eq(&tag.id))
						.get_results(db_connection)?;
					diesel::delete(tags::table)
						.filter(tags::id.eq(&tag.id))
						.execute(db_connection)?;
					let affected_log_entries: Vec<String> = removed_entry_tags
						.iter()
						.map(|entry_tag| entry_tag.log_entry.clone())
						.collect();
					let affected_log_entries: Vec<EventLogEntryDb> = event_log::table
						.filter(event_log::id.eq_any(&affected_log_entries))
						.load(db_connection)?;

					let mut all_events: Vec<EventDb> = events::table.load(db_connection)?;
					let all_events: Vec<Event> = all_events.drain(..).map(|event| event.into()).collect();

					let mut log_entries: Vec<(Event, EventLogEntry)> = Vec::with_capacity(affected_log_entries.len());
					for log_entry in affected_log_entries {
						let tags: Vec<String> = event_log_tags::table
							.filter(event_log_tags::log_entry.eq(&log_entry.id))
							.select(event_log_tags::tag)
							.load(db_connection)?;
						let mut tags: Vec<TagDb> = tags::table.filter(tags::id.eq_any(&tags)).load(db_connection)?;
						let tags: Vec<Tag> = tags.drain(..).map(|tag| tag.into()).collect();
						let event = all_events
							.iter()
							.find(|event| event.id == log_entry.event)
							.unwrap()
							.clone();
						let editor: Option<User> = if let Some(editor_id) = log_entry.editor.as_ref() {
							Some(users::table.find(editor_id).first(db_connection)?)
						} else {
							None
						};
						let editor: Option<UserData> = editor.map(|editor| editor.into());

						log_entries.push((
							event,
							EventLogEntry {
								id: log_entry.id,
								start_time: log_entry.start_time,
								end_time: log_entry.end_time,
								entry_type: log_entry.entry_type,
								description: log_entry.description,
								media_link: log_entry.media_link,
								submitter_or_winner: log_entry.submitter_or_winner,
								tags,
								make_video: log_entry.make_video,
								notes_to_editor: log_entry.notes_to_editor,
								editor_link: log_entry.editor_link,
								editor,
								video_link: log_entry.video_link,
								highlighted: log_entry.highlighted,
								parent: log_entry.parent,
								created_at: log_entry.created_at,
								manual_sort_key: log_entry.manual_sort_key,
							},
						));
					}

					Ok(log_entries)
				});
				match tx_result {
					Ok(entries) => entries,
					Err(error) => {
						tide::log::error!("A database error occurred removing a tag: {}", error);
						return;
					}
				}
			};

			let subscription_manager = subscription_manager.lock().await;
			let admin_message = SubscriptionData::AdminTagsUpdate(AdminTagData::RemoveTag(tag.clone()));
			let send_result = subscription_manager.broadcast_admin_tags_message(admin_message).await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send tag update message for tag removal: {}", error);
			}
			let available_message = SubscriptionData::AvailableTagsUpdate(AvailableTagData::RemoveTag(tag.clone()));
			let send_result = subscription_manager
				.broadcast_available_tags_message(available_message)
				.await;
			if let Err(error) = send_result {
				tide::log::error!("Failed to send tag update message for tag removal: {}", error);
			}

			for (log_entry_event, log_entry) in event_update_data {
				let event_id = log_entry_event.id.clone();
				let event_message = SubscriptionData::EventUpdate(
					log_entry_event,
					Box::new(EventSubscriptionData::UpdateLogEntry(log_entry)),
				);
				let send_result = subscription_manager
					.broadcast_event_message(&event_id, event_message)
					.await;
				if let Err(error) = send_result {
					tide::log::error!("Failed to broadcast event log entry update for tag removal: {}", error);
				}
			}
		}
	}
}
