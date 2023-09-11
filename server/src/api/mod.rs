use crate::models::Application;
use crate::schema::applications;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use tide::{Request, Server, StatusCode};

mod structures;

mod event_by_name;
use event_by_name::event_by_name;

mod event_log_list;
use event_log_list::event_log_list;

mod list_events;
use list_events::list_events;

pub fn add_routes(app: &mut Server<()>, db_connection: Arc<Mutex<PgConnection>>) -> miette::Result<()> {
	app.at("/api/events").get({
		let db_connection = Arc::clone(&db_connection);
		move |request| list_events(request, Arc::clone(&db_connection))
	});
	app.at("/api/event_by_name/:name").get({
		let db_connection = Arc::clone(&db_connection);
		move |request| event_by_name(request, Arc::clone(&db_connection))
	});
	app.at("/api/event/:id/log").get({
		let db_connection = Arc::clone(&db_connection);
		move |request| event_log_list(request, Arc::clone(&db_connection))
	});

	Ok(())
}

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

async fn check_application(
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
