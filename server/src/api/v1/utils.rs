// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::models::{Application, EventLogEntry, EventLogHistoryEntry, EventLogHistoryTag, EventLogTag};
use crate::schema::{applications, event_log_history, event_log_history_tags, event_log_tags};
use chrono::Utc;
use diesel::prelude::*;
use tide::{Request, StatusCode};

#[derive(Debug)]
enum RequestApplicationError {
	NoToken,
	InvalidToken,
}

async fn get_requesting_application(
	request: &Request<()>,
	db_connection: &mut PgConnection,
) -> Result<Application, RequestApplicationError> {
	let auth_token_header = request.header("Authorization");

	match auth_token_header {
		Some(token_header) => {
			let token_header_value = token_header.last();
			applications::table
				.filter(applications::auth_key.eq(token_header_value.as_str()))
				.first(db_connection)
				.map_err(|_| RequestApplicationError::InvalidToken)
		}
		None => Err(RequestApplicationError::NoToken),
	}
}

pub async fn check_application(
	request: &Request<()>,
	db_connection: &mut PgConnection,
) -> Result<Application, tide::Error> {
	let application_result = get_requesting_application(request, db_connection).await;
	match application_result {
		Ok(application) => Ok(application),
		Err(RequestApplicationError::InvalidToken) => Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized"),
		)),
		Err(RequestApplicationError::NoToken) => Err(tide::Error::new(
			StatusCode::BadRequest,
			anyhow::Error::msg("Not authorized"),
		)),
	}
}

pub fn update_history(db_connection: &mut PgConnection, entry: EventLogEntry, application_id: &str) -> QueryResult<()> {
	let tags: Vec<EventLogTag> = event_log_tags::table
		.filter(event_log_tags::log_entry.eq(&entry.id))
		.load(db_connection)?;

	let history_entry = EventLogHistoryEntry {
		id: cuid2::create_id(),
		log_entry: entry.id,
		edit_time: Utc::now(),
		edit_user: None,
		edit_application: Some(application_id.to_owned()),
		start_time: entry.start_time,
		end_time: entry.end_time,
		entry_type: entry.entry_type,
		description: entry.description,
		media_links: entry.media_links,
		submitter_or_winner: entry.submitter_or_winner,
		notes_to_editor: entry.notes_to_editor,
		editor: entry.editor,
		video_link: entry.video_link,
		parent: entry.parent,
		deleted_by: entry.deleted_by,
		created_at: entry.created_at,
		manual_sort_key: entry.manual_sort_key,
		video_processing_state: entry.video_processing_state,
		video_errors: entry.video_errors,
		poster_moment: entry.poster_moment,
		video_edit_state: entry.video_edit_state,
		missing_giveaway_information: entry.missing_giveaway_information,
		end_time_incomplete: entry.end_time_incomplete,
	};
	let history_tags: Vec<EventLogHistoryTag> = tags
		.iter()
		.map(|entry_tag| EventLogHistoryTag {
			history_log_entry: history_entry.id.clone(),
			tag: entry_tag.tag.clone(),
		})
		.collect();
	diesel::insert_into(event_log_history::table)
		.values(history_entry)
		.execute(db_connection)?;
	diesel::insert_into(event_log_history_tags::table)
		.values(history_tags)
		.execute(db_connection)?;

	Ok(())
}
