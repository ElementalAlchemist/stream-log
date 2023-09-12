use super::structures::event::Event as EventApi;
use super::utils::check_application;
use crate::models::Event as EventDb;
use crate::schema::events;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use http_types::mime;
use tide::{Request, Response, StatusCode};

/// GET /api/events
/// Gets a list of events in the database.
pub async fn list_events(request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
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
