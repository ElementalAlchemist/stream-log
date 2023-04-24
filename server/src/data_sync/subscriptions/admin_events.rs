use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::Event as EventDb;
use crate::schema::events;
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_events(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	let mut db_connection = db_connection.lock().await;
	let events: QueryResult<Vec<EventDb>> = events::table.load(&mut *db_connection);
	let events: Vec<Event> = match events {
		Ok(mut events) => events.drain(..).map(|event| event.into()).collect(),
		Err(error) => {
			tide::log::error!("A database error occurred getting the admin events list: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminEvents,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let mut subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_event_subscription(user, conn_update_tx.clone())
		.await;

	let message =
		FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminEvents(events)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}
