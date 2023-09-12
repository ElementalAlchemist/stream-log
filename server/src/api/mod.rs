use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use tide::Server;

mod v1;
use v1::add_routes as add_v1_routes;

pub fn add_routes(app: &mut Server<()>, db_connection: Arc<Mutex<PgConnection>>) -> miette::Result<()> {
	add_v1_routes(app, db_connection)
}
