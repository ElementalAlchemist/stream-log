// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::structures::event::Event as EventApi;
use super::utils::check_application;
use crate::database::handle_lost_db_connection;
use crate::models::Event as EventDb;
use crate::schema::events;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use http_types::mime;
use tide::{Request, Response, StatusCode};

/// GET /api/v1/event_by_name/:name
///
/// Gets the event with a particular name. Returns the [Event](EventApi) object associated with that event.
pub async fn event_by_name(
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

	let event_name = request.param("name")?;
	tide::log::info!("API searching for event: {}", event_name);
	let event: QueryResult<EventDb> = events::table
		.filter(events::name.eq(event_name))
		.first(&mut *db_connection);
	match event {
		Ok(event) => {
			let event = EventApi {
				id: event.id,
				name: event.name,
			};
			let event_json = match serde_json::to_string(&event) {
				Ok(data) => data,
				Err(error) => {
					tide::log::error!("API error serializing event: {}", error);
					return Err(tide::Error::new(
						StatusCode::InternalServerError,
						anyhow::Error::msg("Failed to generate the response."),
					));
				}
			};
			Ok(Response::builder(StatusCode::Ok)
				.body(event_json)
				.content_type(mime::JSON)
				.build())
		}
		Err(error) => {
			if let diesel::result::Error::NotFound = error {
				Err(tide::Error::new(
					StatusCode::NotFound,
					anyhow::Error::msg("No event with that name"),
				))
			} else {
				Err(tide::Error::new(
					StatusCode::InternalServerError,
					anyhow::Error::msg("Database error"),
				))
			}
		}
	}
}
