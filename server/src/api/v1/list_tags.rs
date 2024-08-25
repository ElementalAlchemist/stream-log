// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::structures::tag::{Tag as TagApi, TagPlaylist};
use super::utils::check_application;
use crate::database::handle_lost_db_connection;
use crate::models::{Event as EventDb, Tag as TagDb};
use crate::schema::{events, tags};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use http_types::mime;
use tide::{Request, Response, StatusCode};

/// GET /api/v1/event/:id/tags
///
/// Gets the list of tags available for an event. Responds with the list of [Tag](TagApi) objects as an array.
pub async fn list_tags(
	request: Request<()>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
) -> tide::Result {
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
		Err(diesel::result::Error::NotFound) => {
			return Err(tide::Error::new(
				StatusCode::NotFound,
				anyhow::Error::msg("No such event"),
			))
		}
		Err(error) => {
			tide::log::error!("API error loading event: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			));
		}
	};

	let tags: QueryResult<Vec<TagDb>> = tags::table
		.filter(tags::for_event.eq(&event.id))
		.load(&mut *db_connection);
	let tags: Vec<TagApi> = match tags {
		Ok(tags) => tags
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
				TagApi {
					id: tag.id,
					tag: tag.tag,
					description: tag.description,
					playlist,
				}
			})
			.collect(),
		Err(error) => {
			tide::log::error!("API error loading event tags: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			));
		}
	};

	let tag_json = match serde_json::to_string(&tags) {
		Ok(json) => json,
		Err(error) => {
			tide::log::error!("API error occurred serializing event tags: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Failed to generate the response"),
			));
		}
	};
	Ok(Response::builder(StatusCode::Ok)
		.body(tag_json)
		.content_type(mime::JSON)
		.build())
}
