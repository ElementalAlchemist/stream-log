use super::utils::{check_application, update_history};
use crate::data_sync::SubscriptionManager;
use crate::models::{Event as EventDb, EventLogEntry as EventLogEntryDb, Tag as TagDb, User};
use crate::schema::{event_log, event_log_tags, events, tags, users};
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::SubscriptionData;
use tide::{Request, Response, StatusCode};

/// POST /api/v1/entry/:id/video_errors
///
/// Sets the video errors for the given entry. Request body must be the text of the error. Pass an empty body to clear
/// the error string for the entry.
pub async fn set_video_errors(
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

	let video_errors = request.body_string().await?;
	let entry_id = request.param("id")?;
	let update_result: QueryResult<(Event, EventLogEntry)> = db_connection.transaction(|db_connection| {
		let entry: EventLogEntryDb = diesel::update(event_log::table)
			.filter(event_log::id.eq(entry_id).and(event_log::deleted_by.is_null()))
			.set(event_log::video_errors.eq(video_errors))
			.get_result(db_connection)?;
		update_history(db_connection, entry.clone(), &application.id)?;

		let entry_tags: Vec<TagDb> = tags::table
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
			end_time: entry.end_time,
			entry_type: entry.entry_type,
			description: entry.description,
			media_links: entry.media_links.into_iter().flatten().collect(),
			submitter_or_winner: entry.submitter_or_winner,
			tags: entry_tags.into_iter().map(|tag| tag.into()).collect(),
			notes_to_editor: entry.notes_to_editor,
			editor: editor.map(|editor| editor.into()),
			video_link: entry.video_link,
			parent: entry.parent,
			created_at: entry.created_at,
			manual_sort_key: entry.manual_sort_key,
			video_state: entry.video_state.map(|state| state.into()),
			video_errors: entry.video_errors,
			poster_moment: entry.poster_moment,
			video_edit_state: entry.video_edit_state.into(),
			marked_incomplete: entry.marked_incomplete,
		};
		Ok((event, entry))
	});

	drop(db_connection);

	match update_result {
		Ok((event, entry)) => {
			let subscription_manager = subscription_manager.lock().await;
			let event_id = event.id.clone();
			let message =
				SubscriptionData::EventUpdate(event, Box::new(EventSubscriptionData::UpdateLogEntry(entry, None)));
			if let Err(error) = subscription_manager.broadcast_event_message(&event_id, message).await {
				tide::log::error!(
					"Failed to broadcast event log update after API video errors update: {}",
					error
				);
			}
			Ok(Response::builder(StatusCode::Ok).build())
		}
		Err(diesel::result::Error::NotFound) => Ok(Response::builder(StatusCode::NotFound).build()),
		Err(error) => {
			tide::log::error!("API error setting video error: {}", error);
			Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			))
		}
	}
}
