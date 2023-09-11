use super::check_application;
use super::structures::entry_type::EntryType as EntryTypeApi;
use super::structures::event_log_entry::EventLogEntry as EventLogEntryApi;
use super::structures::event_log_response::EventLogResponse;
use super::structures::user::User as UserApi;
use crate::models::{EntryType as EntryTypeDb, Event as EventDb, EventLogEntry as EventLogEntryDb, User as UserDb};
use crate::schema::{entry_types, event_log, events, users};
use async_std::sync::{Arc, Mutex};
use chrono::Utc;
use diesel::prelude::*;
use http_types::mime;
use std::collections::{HashMap, HashSet};
use tide::{Request, Response, StatusCode};

/// GET /api/event/:id/log
/// Gets all events in the event log for the specified event. Pass an event using the ID.
pub async fn event_log_list(request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
	let application = check_application(&request, &mut db_connection).await?;
	if !application.read_log {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	let event_id = request.param("id")?;
	let event: QueryResult<EventDb> = events::table.find(event_id).first(&mut *db_connection);
	if let Err(error) = event {
		if let diesel::result::Error::NotFound = error {
			return Err(tide::Error::new(
				StatusCode::NotFound,
				anyhow::Error::msg("No such event"),
			));
		}
		tide::log::error!("API error loading event: {}", error);
		return Err(tide::Error::new(
			StatusCode::InternalServerError,
			anyhow::Error::msg("Database error"),
		));
	};

	let retrieved_time = Utc::now();
	let event_log: QueryResult<Vec<EventLogEntryDb>> = event_log::table
		.filter(event_log::event.eq(event_id).and(event_log::deleted_by.is_null()))
		.order_by((
			event_log::start_time.asc(),
			event_log::manual_sort_key.asc(),
			event_log::created_at.asc(),
		))
		.load(&mut *db_connection);
	let mut event_log: Vec<EventLogEntryDb> = match event_log {
		Ok(event_log) => event_log,
		Err(error) => {
			tide::log::error!("API error loading event log: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			));
		}
	};

	let mut entry_type_ids: HashSet<String> = event_log.iter().map(|log_entry| log_entry.entry_type.clone()).collect();
	let entry_type_ids: Vec<String> = entry_type_ids.drain().collect();
	let entry_types: HashMap<String, EntryTypeApi> = if entry_type_ids.is_empty() {
		HashMap::new()
	} else {
		let entry_types: QueryResult<Vec<EntryTypeDb>> = entry_types::table
			.filter(entry_types::id.eq_any(&entry_type_ids))
			.load(&mut *db_connection);
		match entry_types {
			Ok(mut entry_types) => entry_types
				.drain(..)
				.map(|entry_type| {
					(
						entry_type.id.clone(),
						EntryTypeApi {
							id: entry_type.id,
							name: entry_type.name,
							color_red: entry_type.color_red.try_into().unwrap(),
							color_green: entry_type.color_green.try_into().unwrap(),
							color_blue: entry_type.color_blue.try_into().unwrap(),
						},
					)
				})
				.collect(),
			Err(error) => {
				tide::log::error!("API error loading entry types: {}", error);
				return Err(tide::Error::new(
					StatusCode::InternalServerError,
					anyhow::Error::msg("Database error"),
				));
			}
		}
	};

	let mut editor_ids: HashSet<String> = event_log
		.iter()
		.filter_map(|log_entry| log_entry.editor.clone())
		.collect();
	let editor_ids: Vec<String> = editor_ids.drain().collect();
	let editors: HashMap<String, UserApi> = if editor_ids.is_empty() {
		HashMap::new()
	} else {
		let editors: QueryResult<Vec<UserDb>> = users::table
			.filter(users::id.eq_any(&editor_ids))
			.load(&mut *db_connection);
		match editors {
			Ok(mut editors) => editors
				.drain(..)
				.map(|user| {
					(
						user.id.clone(),
						UserApi {
							id: user.id,
							username: user.name,
							color_red: user.color_red.try_into().unwrap(),
							color_green: user.color_green.try_into().unwrap(),
							color_blue: user.color_blue.try_into().unwrap(),
						},
					)
				})
				.collect(),
			Err(error) => {
				tide::log::error!("API error loading event log editors: {}", error);
				return Err(tide::Error::new(
					StatusCode::InternalServerError,
					anyhow::Error::msg("Database error"),
				));
			}
		}
	};

	let event_log: Vec<EventLogEntryApi> = event_log
		.drain(..)
		.map(|entry| EventLogEntryApi {
			id: entry.id,
			start_time: entry.start_time,
			end_time: entry.end_time,
			entry_type: (*entry_types.get(&entry.entry_type).unwrap()).clone(),
			description: entry.description.clone(),
			media_link: entry.media_link.clone(),
			submitter_or_winner: entry.submitter_or_winner.clone(),
			notes_to_editor: entry.notes_to_editor.clone(),
			editor_link: entry.editor_link.clone(),
			editor: entry
				.editor
				.as_ref()
				.map(|editor_id| editors.get(editor_id).unwrap().clone()),
			video_link: entry.video_link.clone(),
			parent: entry.parent.clone(),
			manual_sort_key: entry.manual_sort_key,
			poster_moment: entry.poster_moment,
			marked_incomplete: entry.marked_incomplete,
		})
		.collect();

	let event_log_response = EventLogResponse {
		event_log,
		retrieved_time,
	};
	let event_log_json = match serde_json::to_string(&event_log_response) {
		Ok(json) => json,
		Err(error) => {
			tide::log::error!("API error occurred serializing the event log: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Failed to generate the response"),
			));
		}
	};
	Ok(Response::builder(StatusCode::Ok)
		.body(event_log_json)
		.content_type(mime::JSON)
		.build())
}
