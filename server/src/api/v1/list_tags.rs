use super::structures::tag::Tag as TagApi;
use super::utils::check_application;
use crate::models::{Event as EventDb, Tag as TagDb};
use crate::schema::{events, tags};
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use http_types::mime;
use tide::{Request, Response, StatusCode};

/// GET /api/v1/event/:id/tags
///
/// Gets the list of tags available for an event. Responds with the list of [Tag](TagApi) objects as an array.
pub async fn list_tags(request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
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
			.map(|tag| TagApi {
				id: tag.id,
				tag: tag.tag,
				description: tag.description,
				playlist: if tag.playlist.is_empty() {
					None
				} else {
					Some(tag.playlist)
				},
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
