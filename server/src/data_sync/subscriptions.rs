use crate::models::User;
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use tide_websockets::WebSocketConnection;

pub async fn subscribe_to_event(
	db_connection: Arc<Mutex<PgConnection>>,
	stream: Arc<Mutex<WebSocketConnection>>,
	user: &User,
	event_id: &str,
) {
	// TODO
}

pub async fn unsubscribe_all(stream: Arc<Mutex<WebSocketConnection>>, user: &User) {
	// TODO
}
