use super::utils::{check_application, update_history};
use crate::models::EventLogEntry;
use crate::schema::event_log;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use tide::{Request, Response, StatusCode};

/// Sets the published video link for an event log entry. The body of the request is the link that's associated with the
/// log entry.
pub async fn set_video_link(mut request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
	let application = check_application(&request, &mut db_connection).await?;
	if !application.write_links {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	let video_link = request.body_string().await?;
	if video_link.is_empty() {
		return Ok(Response::builder(StatusCode::BadRequest).build());
	}
	update_video_link(&request, &mut db_connection, &application.id, Some(video_link))
}

/// Deletes the published video link for an event log entry.
pub async fn delete_video_link(request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
	let application = check_application(&request, &mut db_connection).await?;
	if !application.write_links {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	update_video_link(&request, &mut db_connection, &application.id, None)
}

fn update_video_link(
	request: &Request<()>,
	db_connection: &mut PgConnection,
	application_id: &str,
	video_link: Option<String>,
) -> tide::Result {
	let entry_id = request.param("id")?;
	let update_result: QueryResult<()> = db_connection.transaction(|db_connection| {
		let entry: EventLogEntry = diesel::update(event_log::table)
			.filter(event_log::id.eq(entry_id).and(event_log::deleted_by.is_null()))
			.set(event_log::video_link.eq(video_link))
			.get_result(db_connection)?;
		update_history(db_connection, entry, application_id)?;

		Ok(())
	});

	let response = match update_result {
		Ok(_) => Response::builder(StatusCode::Ok).build(),
		Err(diesel::result::Error::NotFound) => Response::builder(StatusCode::NotFound).build(),
		Err(error) => {
			tide::log::error!("Database error setting editor link: {}", error);
			Response::builder(StatusCode::InternalServerError)
				.body("Database error")
				.build()
		}
	};
	Ok(response)
}
