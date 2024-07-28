// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::utils::{check_application, update_history};
use crate::data_sync::SubscriptionManager;
use crate::models::{Event as EventDb, EventLogEntry as EventLogEntryDb, Tag as TagDb, User};
use crate::schema::{event_log, event_log_tags, events, tags, users};
use async_std::sync::{Arc, Mutex, MutexGuard};
use diesel::prelude::*;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::SubscriptionData;
use tide::{Request, Response, StatusCode};

/// POST /api/v1/entry/:id/video
///
/// Sets the published video link for an event log entry. The body of the request is the link that's associated with the
/// log entry.
pub async fn set_video_link(
	mut request: Request<()>,
	db_connection: Arc<Mutex<PgConnection>>,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
	let application = check_application(&request, &mut db_connection).await?;
	if !application.write_links {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	let video_link = request.body_string().await?;
	if video_link.is_empty() {
		return Ok(Response::builder(StatusCode::BadRequest).build());
	}
	update_video_link(
		&request,
		db_connection,
		subscription_manager,
		&application.id,
		Some(video_link),
	)
	.await
}

/// DELETE /api/v1/entry/:id/video
///
/// Deletes the published video link for an event log entry.
pub async fn delete_video_link(
	request: Request<()>,
	db_connection: Arc<Mutex<PgConnection>>,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
	let application = check_application(&request, &mut db_connection).await?;
	if !application.write_links {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	update_video_link(&request, db_connection, subscription_manager, &application.id, None).await
}

async fn update_video_link(
	request: &Request<()>,
	mut db_connection: MutexGuard<'_, PgConnection>,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
	application_id: &str,
	video_link: Option<String>,
) -> tide::Result {
	let entry_id = request.param("id")?;
	let update_result: QueryResult<(Event, EventLogEntry)> = db_connection.transaction(|db_connection| {
		let entry: EventLogEntryDb = diesel::update(event_log::table)
			.filter(event_log::id.eq(entry_id).and(event_log::deleted_by.is_null()))
			.set(event_log::video_link.eq(video_link))
			.get_result(db_connection)?;
		update_history(db_connection, entry.clone(), application_id)?;

		let end_time = entry.end_time_data();

		let tags: Vec<TagDb> = tags::table
			.filter(
				tags::id.eq_any(
					event_log_tags::table
						.filter(event_log_tags::log_entry.eq(&entry.id))
						.select(event_log_tags::tag),
				),
			)
			.load(db_connection)?;
		let editor: Option<User> = if let Some(editor) = entry.editor.as_ref() {
			Some(users::table.find(editor).first(db_connection)?)
		} else {
			None
		};

		let event: EventDb = events::table.find(&entry.event).first(db_connection)?;
		let event: Event = event.into();

		let entry = EventLogEntry {
			id: entry.id,
			start_time: entry.start_time,
			end_time,
			entry_type: entry.entry_type,
			description: entry.description,
			media_links: entry.media_links.into_iter().flatten().collect(),
			submitter_or_winner: entry.submitter_or_winner,
			tags: tags.into_iter().map(|tag| tag.into()).collect(),
			notes_to_editor: entry.notes_to_editor,
			editor: editor.map(|editor| editor.into()),
			video_link: entry.video_link,
			parent: entry.parent,
			created_at: entry.created_at,
			manual_sort_key: entry.manual_sort_key,
			video_processing_state: entry.video_processing_state.map(|state| state.into()),
			video_errors: entry.video_errors,
			poster_moment: entry.poster_moment,
			video_edit_state: entry.video_edit_state.into(),
			marked_incomplete: entry.marked_incomplete,
		};

		Ok((event, entry))
	});

	drop(db_connection);

	let response = match update_result {
		Ok((event, entry)) => {
			let subscription_manager = subscription_manager.lock().await;
			let event_id = event.id.clone();
			let message =
				SubscriptionData::EventUpdate(event, Box::new(EventSubscriptionData::UpdateLogEntry(entry, None)));
			if let Err(error) = subscription_manager.broadcast_event_message(&event_id, message).await {
				tide::log::error!("Failed to broadcast entry update for API video link update: {}", error);
			}

			Response::builder(StatusCode::Ok).build()
		}
		Err(diesel::result::Error::NotFound) => Response::builder(StatusCode::NotFound).build(),
		Err(error) => {
			tide::log::error!("Database error setting editor link: {}", error);
			Response::builder(StatusCode::InternalServerError)
				.body("Database error")
				.build()
		}
	};
	Ok(response)
}
