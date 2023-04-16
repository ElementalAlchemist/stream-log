use async_std::sync::{Arc, Mutex};
use stream_log_shared::messages::subscriptions::SubscriptionData;
use stream_log_shared::messages::user::UserSubscriptionUpdate;
use stream_log_shared::messages::FromServerMessage;
use tide_websockets::WebSocketConnection;

/// Manages the login subscription for a user
pub struct UserSubscription {
	connection: Arc<Mutex<WebSocketConnection>>,
}

impl UserSubscription {
	pub fn new(connection: Arc<Mutex<WebSocketConnection>>) -> Self {
		Self { connection }
	}

	pub async fn send_message(&self, message: UserSubscriptionUpdate) -> tide::Result<()> {
		let message = FromServerMessage::SubscriptionMessage(Box::new(SubscriptionData::UserUpdate(message)));
		let connection = self.connection.lock().await;
		connection.send_json(&message).await
	}
}
