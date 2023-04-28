use crate::data_sync::connection::ConnectionUpdate;
use crate::data_sync::{HandleConnectionError, SubscriptionManager};
use crate::models::User;
use crate::schema::users;
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, FromServerMessage};

pub async fn subscribe_to_admin_users(
	db_connection: Arc<Mutex<PgConnection>>,
	conn_update_tx: Sender<ConnectionUpdate>,
	user: &UserData,
	subscription_manager: Arc<Mutex<SubscriptionManager>>,
) -> Result<(), HandleConnectionError> {
	if !user.is_admin {
		let message =
			FromServerMessage::SubscriptionFailure(SubscriptionType::AdminUsers, SubscriptionFailureInfo::NotAllowed);
		conn_update_tx
			.send(ConnectionUpdate::SendData(Box::new(message)))
			.await?;
		return Ok(());
	}

	let mut db_connection = db_connection.lock().await;
	let all_users: QueryResult<Vec<User>> = users::table.load(&mut *db_connection);
	let mut all_users = match all_users {
		Ok(users) => users,
		Err(error) => {
			tide::log::error!("A database error occurred getting the user list: {}", error);
			let message = FromServerMessage::SubscriptionFailure(
				SubscriptionType::AdminUsers,
				SubscriptionFailureInfo::Error(DataError::DatabaseError),
			);
			conn_update_tx
				.send(ConnectionUpdate::SendData(Box::new(message)))
				.await?;
			return Ok(());
		}
	};

	let subscription_manager = subscription_manager.lock().await;
	subscription_manager
		.add_admin_user_subscription(user, conn_update_tx.clone())
		.await;

	let all_user_data: Vec<UserData> = all_users.drain(..).map(|user| user.into()).collect();
	let message =
		FromServerMessage::InitialSubscriptionLoad(Box::new(InitialSubscriptionLoadData::AdminUsers(all_user_data)));
	conn_update_tx
		.send(ConnectionUpdate::SendData(Box::new(message)))
		.await?;

	Ok(())
}
