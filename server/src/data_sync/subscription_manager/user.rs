use crate::models::Permission;
use async_std::channel::Sender;
use async_std::sync::{Arc, Mutex};
use stream_log_shared::messages::subscriptions::SubscriptionData;
use stream_log_shared::messages::user::{UserData, UserSubscriptionUpdate};
use stream_log_shared::messages::FromServerMessage;
use tide_websockets::WebSocketConnection;

/// Manages the login subscription for a user
pub struct UserSubscription {
	connection: Arc<Mutex<WebSocketConnection>>,
	server_channel: Sender<UserDataUpdate>,
}

impl UserSubscription {
	pub fn new(connection: Arc<Mutex<WebSocketConnection>>, server_channel: Sender<UserDataUpdate>) -> Self {
		Self {
			connection,
			server_channel,
		}
	}

	pub async fn send_message(&self, message: UserSubscriptionUpdate) -> tide::Result<()> {
		let message = FromServerMessage::SubscriptionMessage(Box::new(SubscriptionData::UserUpdate(message)));
		let connection = self.connection.lock().await;
		connection.send_json(&message).await
	}
}

pub enum UserDataUpdate {
	User(UserData),
	EventPermissions(String, Option<Permission>),
}
