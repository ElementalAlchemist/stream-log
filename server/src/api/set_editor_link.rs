use super::check_application;
use crate::models::{EventLogEntry, EventLogHistoryEntry, EventLogHistoryTag, EventLogTag};
use crate::schema::{event_log, event_log_history, event_log_history_tags, event_log_tags};
use async_std::sync::{Arc, Mutex};
use chrono::Utc;
use diesel::prelude::*;
use tide::{Request, Response, StatusCode};

/// Sets the editor link for an event log entry. The body of the request is the link that's associated with the log
/// entry.
pub async fn set_editor_link(mut request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
	let application = check_application(&request, &mut db_connection).await?;
	if !application.write_links {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	let editor_link = request.body_string().await?;
	if editor_link.is_empty() {
		return Ok(Response::builder(StatusCode::BadRequest).build());
	}
	update_editor_link(&request, &mut db_connection, &application.id, Some(editor_link))
}

/// Deletes the editor link for an event log entry.
pub async fn delete_editor_link(request: Request<()>, db_connection: Arc<Mutex<PgConnection>>) -> tide::Result {
	let mut db_connection = db_connection.lock().await;
	let application = check_application(&request, &mut db_connection).await?;
	if !application.write_links {
		return Err(tide::Error::new(
			StatusCode::Unauthorized,
			anyhow::Error::msg("Not authorized to access this resource."),
		));
	}

	update_editor_link(&request, &mut db_connection, &application.id, None)
}

fn update_editor_link(
	request: &Request<()>,
	db_connection: &mut PgConnection,
	application_id: &str,
	editor_link: Option<String>,
) -> tide::Result {
	let entry_id = request.param("id")?;
	let update_result: QueryResult<()> = db_connection.transaction(|db_connection| {
		let entry: EventLogEntry = diesel::update(event_log::table)
			.filter(event_log::id.eq(entry_id).and(event_log::deleted_by.is_null()))
			.set(event_log::editor_link.eq(editor_link))
			.get_result(db_connection)?;
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
			media_link: entry.media_link,
			submitter_or_winner: entry.submitter_or_winner,
			notes_to_editor: entry.notes_to_editor,
			editor_link: entry.editor_link,
			editor: entry.editor,
			video_link: entry.video_link,
			parent: entry.parent,
			deleted_by: entry.deleted_by,
			created_at: entry.created_at,
			manual_sort_key: entry.manual_sort_key,
			video_state: entry.video_state,
			video_errors: entry.video_errors,
			poster_moment: entry.poster_moment,
			video_edit_state: entry.video_edit_state,
			marked_incomplete: entry.marked_incomplete,
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
