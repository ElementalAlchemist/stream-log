use super::HandleConnectionError;
use crate::models::{Event as EventDb, PermissionGroup as PermissionGroupDb};
use crate::schema::{events, permission_groups};
use crate::websocket_msg::recv_msg;
use async_std::sync::{Arc, Mutex};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use stream_log_shared::messages::admin::{AdminAction, EventList, PermissionGroup, PermissionGroupList};
use stream_log_shared::messages::events::Event as EventWs;
use stream_log_shared::messages::{DataError, DataMessage, SubPageControl};
use tide_websockets::WebSocketConnection;

/// Handles the administration flow for a client.
/// Ensure security is checked before calling this.
pub async fn handle_admin(
	stream: &mut WebSocketConnection,
	db_connection: Arc<Mutex<PgConnection>>,
) -> Result<(), HandleConnectionError> {
	loop {
		let incoming_msg = recv_msg(stream).await;
		let incoming_msg = match incoming_msg {
			Ok(msg) => msg,
			Err(error) => {
				error.log();
				break Err(HandleConnectionError::ConnectionClosed);
			}
		};
		let incoming_msg: SubPageControl<AdminAction> = match serde_json::from_str(&incoming_msg) {
			Ok(msg) => msg,
			Err(error) => {
				tide::log::error!("Received an incorrect message in administration: {}", error);
				break Err(HandleConnectionError::ConnectionClosed);
			}
		};
		match incoming_msg {
			SubPageControl::Event(action) => match action {
				AdminAction::ListEvents => {
					let events: QueryResult<Vec<EventDb>> = {
						let db_connection = db_connection.lock().await;
						events::table.load(&*db_connection)
					};
					let events: Vec<EventWs> = match events {
						Ok(mut events) => events
							.drain(..)
							.map(|event| EventWs {
								id: event.id,
								name: event.name,
							})
							.collect(),
						Err(error) => {
							tide::log::error!("Database error: {}", error);
							let message: DataMessage<EventList> = DataMessage::Err(DataError::DatabaseError);
							stream.send_json(&message).await?;
							continue;
						}
					};
					let event_list = EventList { events };
					stream.send_json(&event_list).await?;
				}
				AdminAction::AddEvent(new_event) => {
					let id = match cuid::cuid() {
						Ok(id) => id,
						Err(error) => {
							tide::log::error!("Failed to generate CUID: {}", error);
							break Err(HandleConnectionError::ConnectionClosed);
						}
					};
					let event = EventDb {
						id,
						name: new_event.name,
					};
					let insert_result = {
						let db_connection = db_connection.lock().await;
						diesel::insert_into(events::table)
							.values(&event)
							.execute(&*db_connection)
					};
					if let Err(error) = insert_result {
						tide::log::error!("Database error: {}", error);
						break Err(HandleConnectionError::ConnectionClosed);
					}
				}
				AdminAction::EditEvent(event) => {
					let update_result = {
						let db_connection = db_connection.lock().await;
						diesel::update(events::table.filter(events::id.eq(event.id)))
							.set(events::name.eq(event.name))
							.execute(&*db_connection)
					};
					if let Err(error) = update_result {
						tide::log::error!("Database error: {}", error);
						break Err(HandleConnectionError::ConnectionClosed);
					}
				}
				AdminAction::ListPermissionGroups => {
					let groups: QueryResult<Vec<PermissionGroupDb>> = {
						let db_connection = db_connection.lock().await;
						permission_groups::table.load(&*db_connection)
					};
					let permission_groups: Vec<PermissionGroup> = match groups {
						Ok(mut groups) => groups
							.drain(..)
							.map(|group| PermissionGroup {
								id: group.id,
								name: group.name,
							})
							.collect(),
						Err(error) => {
							tide::log::error!("Database error: {}", error);
							let message: DataMessage<EventList> = DataMessage::Err(DataError::DatabaseError);
							stream.send_json(&message).await?;
							continue;
						}
					};
					let group_list = PermissionGroupList { permission_groups };
					stream.send_json(&group_list).await?;
				}
			},
			SubPageControl::ReturnFromPage => return Ok(()),
		}
	}
}
