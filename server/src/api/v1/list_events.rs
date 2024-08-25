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

/// GET /api/v1/events
///
/// Gets a list of events in the database. Responds with a list of [Event](EventApi) objects.
pub async fn list_events(
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
			anyhow::Error::msg("Not authorized to access this resource"),
		));
	}

	let events: QueryResult<Vec<EventDb>> = events::table.load(&mut *db_connection);
	let events: Vec<EventApi> = match events {
		Ok(events) => events
			.iter()
			.map(|event| EventApi {
				id: event.id.clone(),
				name: event.name.clone(),
			})
			.collect(),
		Err(error) => {
			tide::log::error!("API error listing events: {}", error);
			return Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			));
		}
	};

	match serde_json::to_string(&events) {
		Ok(events_data) => Ok(Response::builder(StatusCode::Ok)
			.body(events_data)
			.content_type(mime::JSON)
			.build()),
		Err(_) => Err(tide::Error::new(
			StatusCode::InternalServerError,
			anyhow::Error::msg("Failed to generate response"),
		)),
	}
}
