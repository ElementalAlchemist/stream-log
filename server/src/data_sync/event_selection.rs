use super::HandleConnectionError;
use crate::models;
use crate::schema::{events, permission_events, permission_groups, user_permissions};
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use stream_log_shared::messages::events as event_messages;
use stream_log_shared::messages::{DataError, DataMessage};
use tide_websockets::WebSocketConnection;

pub async fn send_events(
	db_connection: &Arc<Mutex<PgConnection>>,
	stream: &mut WebSocketConnection,
	user: &models::User,
) -> Result<(), HandleConnectionError> {
	let user_events: QueryResult<Vec<models::Event>> = {
		let mut db_connection = db_connection.lock().await;
		user_permissions::table
			.filter(user_permissions::user_id.eq(&user.id))
			.inner_join(permission_groups::table)
			.inner_join(permission_events::table.on(permission_groups::id.eq(permission_events::permission_group)))
			.inner_join(events::table.on(permission_events::event.eq(events::id)))
			.select(events::table.default_selection())
			.distinct()
			.load(&mut *db_connection)
	};
	match user_events {
		Ok(results) => {
			let event_selection: Vec<event_messages::Event> = results
				.iter()
				.map(|event| event_messages::Event {
					id: event.id.clone(),
					name: event.name.clone(),
					start_time: event.start_time,
				})
				.collect();
			let event_selection = DataMessage::Ok(event_messages::EventSelection {
				available_events: event_selection,
			});
			stream.send_json(&event_selection).await?;
		}
		Err(error) => {
			tide::log::error!("Database error: {}", error);
			let message: DataMessage<event_messages::EventSelection> = DataMessage::Err(DataError::DatabaseError);
			stream.send_json(&message).await?;
			return Err(HandleConnectionError::ConnectionClosed);
		}
	}
	Ok(())
}
