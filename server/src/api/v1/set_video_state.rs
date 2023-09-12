use super::structures::video_state::VideoState as VideoStateApi;
use super::utils::{check_application, update_history};
use crate::models::{EventLogEntry as EventLogEntryDb, VideoState as VideoStateDb};
use crate::schema::event_log;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use tide::{Request, Response, StatusCode};

/// POST /api/v1/entry/:id/video_state
///
/// Sets the video state for the specified entry. The body of the request must be a valid video state.
pub async fn set_video_state(mut request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
	let application = check_application(&request, &mut db_connection).await?;
	if !application.write_links {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	let video_state = request.body_string().await?;
	let video_state: VideoStateApi = match video_state.parse() {
		Ok(state) => state,
		Err(_) => {
			return Err(tide::Error::new(
				StatusCode::BadRequest,
				anyhow::Error::msg("Unknown state"),
			))
		}
	};

	update_video_state(&request, &mut db_connection, &application.id, Some(video_state))
}

/// DELETE /api/v1/entry/:id/video_state
///
/// Removes the video state for the specified entry.
pub async fn delete_video_state(request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
	let application = check_application(&request, &mut db_connection).await?;
	if !application.write_links {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	update_video_state(&request, &mut db_connection, &application.id, None)
}

fn update_video_state(
	request: &Request<()>,
	db_connection: &mut PgConnection,
	application_id: &str,
	video_state: Option<VideoStateApi>,
) -> tide::Result {
	let event_id = request.param("id")?;
	let update_result: QueryResult<()> = db_connection.transaction(|db_connection| {
		let video_state: Option<VideoStateDb> = video_state.map(|state| state.into());
		let entry: EventLogEntryDb = diesel::update(event_log::table)
			.filter(event_log::id.eq(event_id).and(event_log::deleted_by.is_null()))
			.set(event_log::video_state.eq(video_state))
			.get_result(db_connection)?;
		update_history(db_connection, entry, application_id)?;
		Ok(())
	});

	match update_result {
		Ok(()) => Ok(Response::builder(StatusCode::Ok).build()),
		Err(diesel::result::Error::NotFound) => Ok(Response::builder(StatusCode::NotFound).build()),
		Err(error) => {
			tide::log::error!("API error setting video state: {}", error);
			Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			))
		}
	}
}
