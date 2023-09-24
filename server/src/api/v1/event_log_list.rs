use super::structures::entry_type::EntryType as EntryTypeApi;
use super::structures::event_log_entry::EventLogEntry as EventLogEntryApi;
use super::structures::event_log_response::EventLogResponse;
use super::structures::event_log_section::EventLogSection;
use super::structures::tag::Tag as TagApi;
use super::structures::user::User as UserApi;
use super::utils::check_application;
use crate::models::{
	EntryType as EntryTypeDb, Event as EventDb, EventLogEntry as EventLogEntryDb, EventLogSection as EventLogSectionDb,
	EventLogTag, Tag as TagDb, User as UserDb,
};
use crate::schema::{
	entry_types, event_log, event_log_history, event_log_sections, event_log_tags, events, tags, users,
};
use async_std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use diesel::dsl::max;
use diesel::prelude::*;
use http_types::mime;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use tide::{Request, Response, StatusCode};

#[derive(Deserialize)]
struct QueryParams {
	since: Option<DateTime<Utc>>,
}

/// GET /api/v1/event/:id/log
///
/// Gets all events in the event log for the specified event. Pass an event using the ID. Responds with a list of
/// [EventLogEntry](EventLogEntryApi) objects. If the `since` query argument is passed with an ISO 8601 timestamp, only
/// entries last updated on or after that timestamp are included in the list.
pub async fn event_log_list(request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
	let query_params: QueryParams = request.query()?;

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
	let event_log: QueryResult<Vec<EventLogEntryDb>> = if let Some(edited_since) = query_params.since {
		event_log::table
			.filter(
				event_log::event.eq(event_id).and(event_log::deleted_by.is_null()).and(
					event_log_history::table
						.filter(event_log_history::log_entry.eq(event_log::id))
						.select(max(event_log_history::edit_time))
						.single_value()
						.ge(edited_since),
				),
			)
			.order_by((
				event_log::start_time.asc(),
				event_log::manual_sort_key.asc().nulls_last(),
				event_log::created_at.asc(),
			))
			.load(&mut *db_connection)
	} else {
		event_log::table
			.filter(event_log::event.eq(event_id).and(event_log::deleted_by.is_null()))
			.order_by((
				event_log::start_time.asc(),
				event_log::manual_sort_key.asc().nulls_last(),
				event_log::created_at.asc(),
			))
			.load(&mut *db_connection)
	};
	let event_log: Vec<EventLogEntryDb> = match event_log {
		Ok(event_log) => event_log,
		Err(error) => {
			tide::log::error!("API error loading event log: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			));
		}
	};

	let event_log_sections: QueryResult<Vec<EventLogSectionDb>> = event_log_sections::table
		.filter(event_log_sections::event.eq(&event_id))
		.load(&mut *db_connection);
	let event_log_sections = match event_log_sections {
		Ok(sections) => sections,
		Err(error) => {
			tide::log::error!("API error loading event log sections: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			));
		}
	};
	let event_log_sections_by_start_time: BTreeMap<DateTime<Utc>, EventLogSection> = event_log_sections
		.iter()
		.map(|section| (section.start_time, (*section).clone().into()))
		.collect();

	let entry_type_ids: HashSet<String> = event_log.iter().map(|log_entry| log_entry.entry_type.clone()).collect();
	let entry_type_ids: Vec<String> = entry_type_ids.into_iter().collect();
	let entry_types: HashMap<String, EntryTypeApi> = if entry_type_ids.is_empty() {
		HashMap::new()
	} else {
		let entry_types: QueryResult<Vec<EntryTypeDb>> = entry_types::table
			.filter(entry_types::id.eq_any(&entry_type_ids))
			.load(&mut *db_connection);
		match entry_types {
			Ok(entry_types) => entry_types
				.into_iter()
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

	let editor_ids: HashSet<String> = event_log
		.iter()
		.filter_map(|log_entry| log_entry.editor.clone())
		.collect();
	let editor_ids: Vec<String> = editor_ids.into_iter().collect();
	let editors: HashMap<String, UserApi> = if editor_ids.is_empty() {
		HashMap::new()
	} else {
		let editors: QueryResult<Vec<UserDb>> = users::table
			.filter(users::id.eq_any(&editor_ids))
			.load(&mut *db_connection);
		match editors {
			Ok(editors) => editors
				.into_iter()
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

	let event_log_ids: Vec<String> = event_log.iter().map(|entry| entry.id.clone()).collect();
	let entry_tags: QueryResult<Vec<EventLogTag>> = event_log_tags::table
		.filter(event_log_tags::log_entry.eq_any(event_log_ids))
		.load(&mut *db_connection);
	let entry_tags = match entry_tags {
		Ok(entry_tags) => entry_tags,
		Err(error) => {
			tide::log::error!("API error loading event log tags: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			));
		}
	};
	let tag_ids: Vec<String> = entry_tags.iter().map(|entry_tag| entry_tag.tag.clone()).collect();
	let tags: QueryResult<Vec<TagDb>> = tags::table.filter(tags::id.eq_any(tag_ids)).load(&mut *db_connection);
	let tags = match tags {
		Ok(tags) => tags,
		Err(error) => {
			tide::log::error!("API error loading tags for event log: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			));
		}
	};
	let mut entry_tag_map: HashMap<String, Vec<String>> = HashMap::new();
	for entry_tag in entry_tags {
		entry_tag_map
			.entry(entry_tag.log_entry)
			.or_default()
			.push(entry_tag.tag);
	}
	let tags_by_id: HashMap<String, TagApi> = tags
		.into_iter()
		.map(|tag| {
			(
				tag.id.clone(),
				TagApi {
					id: tag.id,
					tag: tag.tag,
					description: tag.description,
					playlist: if tag.playlist.is_empty() {
						None
					} else {
						Some(tag.playlist)
					},
				},
			)
		})
		.collect();
	let entry_tag_map: HashMap<String, Vec<TagApi>> = entry_tag_map
		.into_iter()
		.map(|(entry_id, tag_ids)| {
			let tags = tag_ids
				.iter()
				.map(|tag_id| tags_by_id.get(tag_id).cloned().unwrap())
				.collect();
			(entry_id, tags)
		})
		.collect();

	let event_log: Vec<EventLogEntryApi> = event_log
		.into_iter()
		.map(|entry| EventLogEntryApi {
			id: entry.id.clone(),
			start_time: entry.start_time,
			end_time: entry.end_time,
			entry_type: (*entry_types.get(&entry.entry_type).unwrap()).clone(),
			description: entry.description.clone(),
			media_links: entry.media_links.iter().filter_map(|link| link.clone()).collect(),
			submitter_or_winner: entry.submitter_or_winner.clone(),
			tags: entry_tag_map.get(&entry.id).cloned().unwrap_or_default(),
			notes_to_editor: entry.notes_to_editor.clone(),
			editor_link: entry.editor_link.clone(),
			editor: entry
				.editor
				.as_ref()
				.map(|editor_id| editors.get(editor_id).unwrap().clone()),
			video_link: entry.video_link.clone(),
			parent: entry.parent.clone(),
			manual_sort_key: entry.manual_sort_key,
			video_edit_state: entry.video_edit_state.into(),
			video_state: entry.video_state.map(|state| state.into()),
			video_errors: entry.video_errors.clone(),
			poster_moment: entry.poster_moment,
			marked_incomplete: entry.marked_incomplete,
			section: event_log_sections_by_start_time
				.range(..=entry.start_time)
				.last()
				.map(|(_, section)| section.clone()),
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
