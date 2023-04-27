use crate::data_sync::{ConnectionUpdate, HandleConnectionError, SubscriptionManager};
use crate::models::Tag as TagDb;
use crate::schema::tags;
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_tags(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message =
			FromServerMessage::SubscriptionFailure(SubscriptionType::AdminTags, SubscriptionFailureInfo::NotAllowed);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = db_connection.lock().await;
	let tags: QueryResult<Vec<TagDb>> = tags::table.load(&mut *db_connection);
	let tags: Vec<Tag> = match tags {
		Ok(mut tags) => tags.drain(..).map(|tag| tag.into()).collect(),
		Err(error) => {
			tide::log::error!(
				"A database error occurred retrieving tag data for admin subscription: {}",
				error
			);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminTags,
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
		.add_admin_tags_subscription(user, conn_update_tx.clone())
		.await;

	let message = FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminTags(tags)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}
