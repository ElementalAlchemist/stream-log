use super::HandleConnectionError;
use crate::models::{Approval, Event as EventDb, User};
use crate::schema::{events, users};
use crate::websocket_msg::recv_msg;
use async_std::sync::{Arc, Mutex};
use diesel::dsl::count;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use stream_log_shared::messages::admin::{AdminAction, EventList, MenuInfo, UnapprovedUsers};
use stream_log_shared::messages::events::Event as EventWs;
use stream_log_shared::messages::user::UserData;
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
				AdminAction::MenuInfo => {
					let unapproved_user_count: QueryResult<i64> = {
						let db_connection = db_connection.lock().await;
						users::table
							.filter(users::account_level.eq(Approval::Unapproved))
							.select(count(users::id))
							.first(&*db_connection)
					};
					let message = match unapproved_user_count {
						Ok(count) => match count.try_into() {
							Ok(count) => {
								let menu_info = MenuInfo {
									unapproved_user_count: count,
								};
								DataMessage::Ok(menu_info)
							}
							Err(error) => {
								tide::log::error!("Error converting database count to unsigned integer? {}", error);
								DataMessage::Err(DataError::ServerError)
							}
						},
						Err(error) => {
							tide::log::error!("Database error: {}", error);
							DataMessage::Err(DataError::DatabaseError)
						}
					};
					stream.send_json(&message).await?;
				}
				AdminAction::UnapprovedUserList => {
					let unapproved_users: QueryResult<Vec<User>> = {
						let db_connection = db_connection.lock().await;
						users::table
							.filter(users::account_level.eq(Approval::Unapproved))
							.load(&*db_connection)
					};
					let message = match unapproved_users {
						Ok(mut users) => {
							let users: Vec<UserData> = users
								.drain(..)
								.map(|user| UserData {
									id: user.id,
									username: user.name,
									approval_level: user.account_level.into(),
								})
								.collect();
							DataMessage::Ok(UnapprovedUsers { users })
						}
						Err(error) => {
							tide::log::error!("Database error: {}", error);
							DataMessage::Err(DataError::DatabaseError)
						}
					};
					stream.send_json(&message).await?;
				}
				AdminAction::ApproveUser(user) => {
					let id = user.id;
					let db_connection = db_connection.lock().await;
					let update_result: QueryResult<()> = db_connection.transaction(|| {
						let current_user: User = users::table.filter(users::id.eq(&id)).first(&*db_connection)?;
						if current_user.account_level == Approval::Unapproved {
							diesel::update(users::table.filter(users::id.eq(&id)))
								.set(users::account_level.eq(Approval::Approved))
								.execute(&*db_connection)?;
						}
						Ok(())
					});
					if let Err(error) = update_result {
						tide::log::error!("Database error: {}", error);
					}
				}
				AdminAction::DenyUser(user) => {
					let id = user.id;
					let db_connection = db_connection.lock().await;
					let update_result: QueryResult<()> = db_connection.transaction(|| {
						let current_user: User = users::table.filter(users::id.eq(&id)).first(&*db_connection)?;
						if current_user.account_level == Approval::Unapproved {
							diesel::update(users::table.filter(users::id.eq(&id)))
								.set(users::account_level.eq(Approval::Denied))
								.execute(&*db_connection)?;
						}
						Ok(())
					});
					if let Err(error) = update_result {
						tide::log::error!("Database error: {}", error);
					}
				}
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
			},
			SubPageControl::ReturnFromPage => return Ok(()),
		}
	}
}
