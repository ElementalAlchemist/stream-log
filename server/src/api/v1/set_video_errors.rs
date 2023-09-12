use super::utils::{check_application, update_history};
use crate::models::EventLogEntry as EventLogEntryDb;
use crate::schema::event_log;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use tide::{Request, Response, StatusCode};

/// POST /api/v1/entry/:id/video_errors
///
/// Sets the video errors for the given entry. Request body must be the text of the error. Pass an empty body to clear
/// the error string for the entry.
pub async fn set_video_errors(mut request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
	let application = check_application(&request, &mut db_connection).await?;
	if !application.write_links {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	let video_errors = request.body_string().await?;
	let entry_id = request.param("id")?;
	let update_result: QueryResult<()> = db_connection.transaction(|db_connection| {
		let entry: EventLogEntryDb = diesel::update(event_log::table)
			.filter(event_log::id.eq(entry_id).and(event_log::deleted_by.is_null()))
			.set(event_log::video_errors.eq(video_errors))
			.get_result(db_connection)?;
		update_history(db_connection, entry, &application.id)?;
		Ok(())
	});

	match update_result {
		Ok(()) => Ok(Response::builder(StatusCode::Ok).build()),
		Err(diesel::result::Error::NotFound) => Ok(Response::builder(StatusCode::NotFound).build()),
		Err(error) => {
			tide::log::error!("API error setting video error: {}", error);
			Err(tide::Error::new(
				StatusCode::InternalServerError,
				anyhow::Error::msg("Database error"),
			))
		}
	}
}
