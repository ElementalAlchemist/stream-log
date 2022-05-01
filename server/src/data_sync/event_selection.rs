use super::HandleConnectionError;
use crate::models;
use crate::schema::{events, roles};
use crate::websocket_msg::recv_msg;
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::events as event_messages;
use stream_log_shared::messages::{DataError, DataMessage};
use tide_websockets::WebSocketConnection;

pub async fn select_event(
	db_connection: Arc<Mutex<PgConnection>>,
	stream: &mut WebSocketConnection,
	user: &models::User,
) -> Result<models::Event, HandleConnectionError> {
	let mut available_events = {
		let user_events: QueryResult<Vec<(models::Role, models::Event)>> = {
			let db_connection = db_connection.lock().await;
			roles::table
				.filter(roles::user_id.eq(&user.id))
				.inner_join(events::table)
				.load(&*db_connection)
		};
		match user_events {
			Ok(mut results) => {
				let event_selection: Vec<event_messages::Event> = results
					.iter()
					.map(|(_, event)| event_messages::Event {
						id: event.id.clone(),
						name: event.name.clone(),
					})
					.collect();
				let event_selection = event_messages::EventSelection {
					available_events: event_selection,
				};
				stream.send_json(&event_selection).await?;
				results.drain(..).map(|(_, event)| (event.id.clone(), event)).collect()
			}
			Err(error) => {
				tide::log::error!("Database error: {}", error);
				let message: DataMessage<event_messages::EventSelection> = DataMessage::Err(DataError::DatabaseError);
				stream.send_json(&message).await?;
				HashMap::new()
			}
		}
	};
	match recv_msg(stream).await {
		Ok(text) => {
			let selected_event: event_messages::Event = match serde_json::from_str(&text) {
				Ok(event) => event,
				Err(error) => {
					tide::log::error!("Received an incorrect message during event selection: {}", error);
					return Err(HandleConnectionError::ConnectionClosed);
				}
			};
			match available_events.remove(&selected_event.id) {
				Some(event) => Ok(event),
				None => {
					tide::log::info!("User selected an invalid or unauthorized event");
					Err(HandleConnectionError::ConnectionClosed)
				}
			}
		}
		Err(error) => {
			error.log();
			Err(HandleConnectionError::ConnectionClosed)
		}
	}
}
