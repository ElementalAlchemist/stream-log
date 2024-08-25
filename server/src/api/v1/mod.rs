// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::data_sync::SubscriptionManager;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use tide::Server;

mod structures;
mod utils;

mod event_by_name;
use event_by_name::event_by_name;

mod event_log_list;
use event_log_list::event_log_list;

mod list_events;
use list_events::list_events;

mod list_tags;
use list_tags::list_tags;

mod set_video_errors;
use set_video_errors::set_video_errors;

mod set_video_link;
use set_video_link::{delete_video_link, set_video_link};

mod set_video_processing_state;
use set_video_processing_state::set_video_processing_state;

pub fn add_routes(
	app: &mut Server<()>,
	db_connection_pool: Pool<ConnectionManager<PgConnection>>,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> miette::Result<()> {
	app.at("/api/v1/events").get({
		let db_connection_pool = db_connection_pool.clone();
		move |request| list_events(request, db_connection_pool.clone())
	});
	app.at("/api/v1/event_by_name/:name").get({
		let db_connection_pool = db_connection_pool.clone();
		move |request| event_by_name(request, db_connection_pool.clone())
	});
	app.at("/api/v1/event/:id/log").get({
		let db_connection_pool = db_connection_pool.clone();
		move |request| event_log_list(request, db_connection_pool.clone())
	});
	app.at("/api/v1/event/:id/tags").get({
		let db_connection_pool = db_connection_pool.clone();
		move |request| list_tags(request, db_connection_pool.clone())
	});
	app.at("/api/v1/entry/:id/video")
		.post({
			let db_connection_pool = db_connection_pool.clone();
			let subscription_manager = Arc::clone(&subscription_manager);
			move |request| set_video_link(request, db_connection_pool.clone(), Arc::clone(&subscription_manager))
		})
		.delete({
			let db_connection_pool = db_connection_pool.clone();
			let subscription_manager = Arc::clone(&subscription_manager);
			move |request| delete_video_link(request, db_connection_pool.clone(), Arc::clone(&subscription_manager))
		});
	app.at("/api/v1/entry/:id/video_processing_state").post({
		let db_connection_pool = db_connection_pool.clone();
		let subscription_manager = Arc::clone(&subscription_manager);
		move |request| {
			set_video_processing_state(request, db_connection_pool.clone(), Arc::clone(&subscription_manager))
		}
	});
	app.at("/api/v1/entry/:id/video_errors").post({
		let subscription_manager = Arc::clone(&subscription_manager);
		move |request| set_video_errors(request, db_connection_pool.clone(), Arc::clone(&subscription_manager))
	});

	Ok(())
}
