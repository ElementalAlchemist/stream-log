use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use tide::Server;

mod structures;
mod utils;

mod event_by_name;
use event_by_name::event_by_name;

mod event_log_list;
use event_log_list::event_log_list;

mod list_events;
use list_events::list_events;

mod set_video_errors;
use set_video_errors::set_video_errors;

mod set_video_link;
use set_video_link::{delete_video_link, set_video_link};

mod set_video_state;
use set_video_state::{delete_video_state, set_video_state};

pub fn add_routes(app: &mut Server<()>, db_connection: Arc<Mutex<PgConnection>>) -> miette::Result<()> {
	app.at("/api/v1/events").get({
		let db_connection = Arc::clone(&db_connection);
		move |request| list_events(request, Arc::clone(&db_connection))
	});
	app.at("/api/v1/event_by_name/:name").get({
		let db_connection = Arc::clone(&db_connection);
		move |request| event_by_name(request, Arc::clone(&db_connection))
	});
	app.at("/api/v1/event/:id/log").get({
		let db_connection = Arc::clone(&db_connection);
		move |request| event_log_list(request, Arc::clone(&db_connection))
	});
	app.at("/api/v1/entry/:id/video")
		.post({
			let db_connection = Arc::clone(&db_connection);
			move |request| set_video_link(request, Arc::clone(&db_connection))
		})
		.delete({
			let db_connection = Arc::clone(&db_connection);
			move |request| delete_video_link(request, Arc::clone(&db_connection))
		});
	app.at("/api/v1/entry/:id/video_state")
		.post({
			let db_connection = Arc::clone(&db_connection);
			move |request| set_video_state(request, Arc::clone(&db_connection))
		})
		.delete({
			let db_connection = Arc::clone(&db_connection);
			move |request| delete_video_state(request, Arc::clone(&db_connection))
		});
	app.at("/api/v1/entry/:id/video_errors").post({
		let db_connection = Arc::clone(&db_connection);
		move |request| set_video_errors(request, Arc::clone(&db_connection))
	});

	Ok(())
}
