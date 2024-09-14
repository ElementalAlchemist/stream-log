// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::structures::entry_type::EntryType as EntryTypeApi;
use super::structures::event_log_entry::{EndTimeData, EventLogEntry as EventLogEntryApi};
use super::structures::event_log_response::EventLogResponse;
use super::structures::event_log_tab::EventLogTab;
use super::structures::tag::{Tag as TagApi, TagPlaylist};
use super::structures::user::User as UserApi;
use super::utils::check_application;
use crate::database::handle_lost_db_connection;
use crate::models::{
	EntryType as EntryTypeDb, Event as EventDb, EventLogEntry as EventLogEntryDb, EventLogTab as EventLogTabDb,
	EventLogTag, Tag as TagDb, User as UserDb,
};
use crate::schema::{entry_types, event_log, event_log_history, event_log_tabs, event_log_tags, events, tags, users};
use chrono::{DateTime, Utc};
use diesel::dsl::max;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
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
/// Gets all events in the event log for the specified event. Pass an event using the ID. Responds with an
/// [EventLogResponse] object. If the `since` query argument is passed with an ISO 8601 timestamp, only
/// entries last updated on or after that timestamp are included in the list. The timestamp provided in the response
/// may be used in subsequent queries to get exactly all of the changes made since the response was generated.
pub async fn event_log_list(
	request: Request<()>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> tide::Result {
	let query_params: QueryParams = request.query()?;

	let mut db_connection = match db_connection_pool.get() {
		Ok(connection) => connection,
		Err(error) => return handle_lost_db_connection(error),
	};
	let application = check_application(&request, &mut db_connection).await?;
	if !application.read_log {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	let event_id = request.param("id")?;
	let event: QueryResult<EventDb> = events::table.find(event_id).first(&mut *db_connection);
	let event: EventDb = match event {
		Ok(event) => event,
		Err(error) => {
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
		}
	};

	let default_event_tab = EventLogTab {
		id: String::new(),
		name: event.first_tab_name,
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

	let mut event_log_by_parent: HashMap<String, Vec<EventLogEntryDb>> = HashMap::new();
	event_log_by_parent.insert(String::new(), Vec::new());
	for entry in event_log.into_iter() {
		let parent_id = entry.parent.clone().unwrap_or_default();
		event_log_by_parent.entry(parent_id).or_default().push(entry);
	}

	let mut event_log = event_log_by_parent.remove("").unwrap();
	let mut event_log_index = 0;
	while event_log_index < event_log.len() {
		let entry_id = &event_log[event_log_index].id;
		if let Some(children) = event_log_by_parent.remove(entry_id) {
			let insert_index = event_log_index + 1;
			event_log.splice(insert_index..insert_index, children);
		}
		event_log_index += 1;
	}

	// If we only got entries modified since a time, we might have orphaned children. Since the order of the output
	// matters less in that scenario, we'll just stuff all those at the end.
	for mut child_entries in event_log_by_parent.into_values() {
		event_log.append(&mut child_entries);
	}

	// Now that we've handled ordering child entries in the event log, it no longer needs to be mutable.
	let event_log = event_log;

	let event_log_tabs: QueryResult<Vec<EventLogTabDb>> = event_log_tabs::table
		.filter(event_log_tabs::event.eq(&event_id))
		.load(&mut *db_connection);
	let event_log_tabs = match event_log_tabs {
		Ok(tabs) => tabs,
		Err(error) => {
			tide::log::error!("API error loading event log tabs: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			));
		}
	};
	let event_log_tabs_by_start_time: BTreeMap<DateTime<Utc>, EventLogTab> = event_log_tabs
		.iter()
		.map(|tab| (tab.start_time, (*tab).clone().into()))
		.collect();

	let entry_type_ids: HashSet<Option<String>> =
		event_log.iter().map(|log_entry| log_entry.entry_type.clone()).collect();
	let entry_type_ids: Vec<String> = entry_type_ids.into_iter().flatten().collect();
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
			let playlist = if let (Some(id), Some(title), Some(shows_in_video_descriptions)) = (
				tag.playlist,
				tag.playlist_title,
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
			(
				tag.id.clone(),
				TagApi {
					id: tag.id,
					tag: tag.tag,
					description: tag.description,
					playlist,
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

	let mut id_to_entry: HashMap<String, EventLogEntryDb> = HashMap::new();
	for log_entry in event_log.iter() {
		id_to_entry.insert(log_entry.id.clone(), log_entry.clone());
	}
	let mut start_time_index: HashMap<String, DateTime<Utc>> = HashMap::new();

	let event_log: Vec<EventLogEntryApi> = event_log
		.into_iter()
		.map(|entry| {
			let end_time = match (entry.end_time, entry.end_time_incomplete) {
				(Some(time), _) => EndTimeData::Time(time),
				(None, true) => EndTimeData::NotEntered,
				(None, false) => EndTimeData::NoTime,
			};

			let editor_link = event.editor_link_format.replace("{id}", &entry.id);
			let editor_link = if editor_link.is_empty() {
				None
			} else {
				Some(editor_link)
			};

			let tab_start_time = if let Some(parent_id) = entry.parent.as_ref() {
				let mut tab_start_time_entry = if let Some(parent_entry) = id_to_entry.get(parent_id) {
					parent_entry.clone()
				} else {
					entry.clone()
				};
				let mut override_start_time: Option<DateTime<Utc>> = None;
				while let Some(parent_id) = tab_start_time_entry.parent.as_ref() {
					if let Some(start_time) = start_time_index.get(parent_id) {
						override_start_time = Some(*start_time);
						start_time_index.insert(entry.id.clone(), *start_time);
						break;
					}
					if let Some(parent_entry) = id_to_entry.get(parent_id) {
						tab_start_time_entry = parent_entry.clone();
					} else {
						let parent_entry: QueryResult<EventLogEntryDb> =
							event_log::table.find(parent_id).first(&mut *db_connection);
						let parent_entry = match parent_entry {
							Ok(entry) => entry,
							Err(_) => break, // We can't go any further or emit an error from here, so we'll just do our best
						};
						tab_start_time_entry = parent_entry;
					}
				}
				if let Some(start_time) = override_start_time {
					start_time
				} else {
					tab_start_time_entry.start_time
				}
			} else {
				entry.start_time
			};

			let entry_type = entry
				.entry_type
				.as_ref()
				.map(|entry_type_id| (entry_types.get(entry_type_id).unwrap()).clone());

			EventLogEntryApi {
				id: entry.id.clone(),
				start_time: entry.start_time,
				end_time,
				entry_type,
				description: entry.description.clone(),
				media_links: entry.media_links.iter().filter_map(|link| link.clone()).collect(),
				submitter_or_winner: entry.submitter_or_winner.clone(),
				tags: entry_tag_map.get(&entry.id).cloned().unwrap_or_default(),
				notes: entry.notes.clone(),
				editor_link,
				editor: entry
					.editor
					.as_ref()
					.map(|editor_id| editors.get(editor_id).unwrap().clone()),
				video_link: entry.video_link.clone(),
				parent: entry.parent.clone(),
				manual_sort_key: entry.manual_sort_key,
				video_edit_state: entry.video_edit_state.into(),
				video_processing_state: entry.video_processing_state.into(),
				video_errors: entry.video_errors.clone(),
				poster_moment: entry.poster_moment,
				missing_giveaway_information: entry.missing_giveaway_information,
				tab: event_log_tabs_by_start_time
					.range(..=tab_start_time)
					.last()
					.map(|(_, tab)| tab.clone())
					.unwrap_or_else(|| default_event_tab.clone()),
			}
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
