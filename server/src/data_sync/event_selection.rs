use super::admin::handle_admin;
use super::HandleConnectionError;
use crate::models;
use crate::schema::{events, permission_events, permission_groups, user_permissions};
use crate::websocket_msg::recv_msg;
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::HashMap;
use stream_log_shared::messages::events as event_messages;
use stream_log_shared::messages::{DataError, DataMessage, PageControl};
use tide_websockets::WebSocketConnection;

async fn send_events(
	db_connection: &Arc<Mutex<PgConnection>>,
	stream: &mut WebSocketConnection,
	user: &models::User,
) -> Result<HashMap<String, models::Event>, HandleConnectionError> {
	let user_events: QueryResult<Vec<models::Event>> = {
		let db_connection = db_connection.lock().await;
		user_permissions::table
			.filter(user_permissions::user_id.eq(&user.id))
			.inner_join(permission_groups::table)
			.inner_join(permission_events::table.on(permission_groups::id.eq(permission_events::permission_group)))
			.inner_join(events::table.on(permission_events::event.eq(events::id)))
			.select(events::table.default_selection())
			.load(&*db_connection)
	};
	let events = match user_events {
		Ok(mut results) => {
			let event_selection: Vec<event_messages::Event> = results
				.iter()
				.map(|event| event_messages::Event {
					id: event.id.clone(),
					name: event.name.clone(),
				})
				.collect();
			let event_selection = DataMessage::Ok(event_messages::EventSelection {
				available_events: event_selection,
			});
			stream.send_json(&event_selection).await?;
			results.drain(..).map(|event| (event.id.clone(), event)).collect()
		}
		Err(error) => {
			tide::log::error!("Database error: {}", error);
			let message: DataMessage<event_messages::EventSelection> = DataMessage::Err(DataError::DatabaseError);
			stream.send_json(&message).await?;
			HashMap::new()
		}
	};
	Ok(events)
}

pub async fn select_event(
	db_connection: Arc<Mutex<PgConnection>>,
	stream: &mut WebSocketConnection,
	user: &models::User,
) -> Result<models::Event, HandleConnectionError> {
	let mut available_events = send_events(&db_connection, stream, user).await?;
	loop {
		match recv_msg(stream).await {
			Ok(text) => {
				let selected_event_action: PageControl<event_messages::Event> = match serde_json::from_str(&text) {
					Ok(event) => event,
					Err(error) => {
						tide::log::error!("Received an incorrect message during event selection: {}", error);
						break Err(HandleConnectionError::ConnectionClosed);
					}
				};
				match selected_event_action {
					PageControl::Admin => {
						if user.is_admin {
							handle_admin(stream, Arc::clone(&db_connection)).await?;
							available_events = send_events(&db_connection, stream, user).await?;
						}
					}
					PageControl::Event(selected_event) => match available_events.remove(&selected_event.id) {
						Some(event) => break Ok(event),
						None => {
							tide::log::info!("User selected an invalid or unauthorized event");
							break Err(HandleConnectionError::ConnectionClosed);
						}
					},
				}
			}
			Err(error) => {
				error.log();
				break Err(HandleConnectionError::ConnectionClosed);
			}
		}
	}
}