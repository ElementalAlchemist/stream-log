use crate::models::User;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use tide_websockets::WebSocketConnection;

pub struct SubscriptionManager {}

impl SubscriptionManager {
	pub fn new() -> Self {
		todo!()
	}

	pub fn shutdown(&mut self) {}

	pub fn subscribe_user_to_event(
		&mut self,
		event_id: String,
		subscribing_user: &User,
		connection: &mut WebSocketConnection,
	) {
	}

	pub fn broadcast_event_message(&mut self, event_id: String, message: EventSubscriptionData) {}

	pub fn unsubscribe_user_from_all(&mut self, user: &User) {}
}
