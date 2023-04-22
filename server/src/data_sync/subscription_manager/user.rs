use crate::data_sync::connection::ConnectionUpdate;
use crate::models::Permission;
use async_std::channel::{SendError, Sender};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::SubscriptionData;
use stream_log_shared::messages::user::{UserData, UserSubscriptionUpdate};
use stream_log_shared::messages::FromServerMessage;

/// Manages the login subscription for a user
pub struct UserSubscription {
	channel: Sender<ConnectionUpdate>,
}

impl UserSubscription {
	pub fn new(channel: Sender<ConnectionUpdate>) -> Self {
		Self { channel }
	}

	pub async fn send_message(&self, message: UserSubscriptionUpdate) -> Result<(), SendError<ConnectionUpdate>> {
		let message = FromServerMessage::SubscriptionMessage(Box::new(SubscriptionData::UserUpdate(message)));
		let message = ConnectionUpdate::SendData(Box::new(message));
		self.channel.send(message).await
	}
}

#[derive(Clone)]
pub enum UserDataUpdate {
	User(UserData),
	EventPermissions(Event, Option<Permission>),
}
