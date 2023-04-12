use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::{Message, WebSocketError};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use stream_log_shared::messages::subscriptions::SubscriptionType;
use stream_log_shared::messages::FromClientMessage;

pub enum SubscriptionError {
	SerializeError(serde_json::Error),
	SendError(WebSocketError),
}

impl From<serde_json::Error> for SubscriptionError {
	fn from(value: serde_json::Error) -> Self {
		Self::SerializeError(value)
	}
}

impl From<WebSocketError> for SubscriptionError {
	fn from(value: WebSocketError) -> Self {
		Self::SendError(value)
	}
}

#[derive(Debug, Default)]
pub struct SubscriptionManager {
	active_subscriptions: HashMap<SubscriptionType, u32>,
	requested_subscriptions: HashMap<SubscriptionType, u32>,
}

impl SubscriptionManager {
	/// Adds a subscription for data. Records that the request was made and sends it to the server.
	pub async fn add_subscription(
		&mut self,
		subscription_type: SubscriptionType,
		stream: &mut SplitSink<WebSocket, Message>,
	) -> Result<(), SubscriptionError> {
		if let Some(use_count) = self.active_subscriptions.get_mut(&subscription_type) {
			*use_count += 1;
		} else if let Some(use_count) = self.requested_subscriptions.get_mut(&subscription_type) {
			*use_count += 1;
		} else {
			let subscription_message = FromClientMessage::StartSubscription(subscription_type.clone());
			let subscription_message_json = serde_json::to_string(&subscription_message)?;
			stream.send(Message::Text(subscription_message_json)).await?;

			self.requested_subscriptions.insert(subscription_type, 1);
		}

		Ok(())
	}

	/// Adds multiple subscriptions at once
	pub async fn add_subscriptions(
		&mut self,
		subscription_types: Vec<SubscriptionType>,
		stream: &mut SplitSink<WebSocket, Message>,
	) -> Result<(), SubscriptionError> {
		for subscription_type in subscription_types {
			self.add_subscription(subscription_type, stream).await?;
		}
		Ok(())
	}

	/// Removes a subscription for data. Sends the unsubscription request to the server.
	pub async fn remove_subscription(
		&mut self,
		subscription_type: SubscriptionType,
		stream: &mut SplitSink<WebSocket, Message>,
	) -> Result<(), SubscriptionError> {
		let send_remove =
			if let Entry::Occupied(mut active_entry) = self.active_subscriptions.entry(subscription_type.clone()) {
				let current_count = *active_entry.get() - 1;
				if current_count == 0 {
					active_entry.remove();
					true
				} else {
					*active_entry.get_mut() = current_count;
					false
				}
			} else if let Entry::Occupied(mut requested_entry) =
				self.requested_subscriptions.entry(subscription_type.clone())
			{
				let current_count = *requested_entry.get() - 1;
				if current_count == 0 {
					requested_entry.remove();
					true
				} else {
					*requested_entry.get_mut() = current_count;
					false
				}
			} else {
				false
			};

		if send_remove {
			let unsubscription_message = FromClientMessage::EndSubscription(subscription_type);
			let unsubscription_message_json = serde_json::to_string(&unsubscription_message)?;
			stream.send(Message::Text(unsubscription_message_json)).await?;
		}

		Ok(())
	}

	/// Removes multiple subscriptions at once
	pub async fn remove_subscriptions(
		&mut self,
		subscription_types: Vec<SubscriptionType>,
		stream: &mut SplitSink<WebSocket, Message>,
	) -> Result<(), SubscriptionError> {
		for subscription_type in subscription_types {
			self.remove_subscription(subscription_type, stream).await?;
		}
		Ok(())
	}

	/// To be called when a subscription confirmation is received from the server. Updates tracking from requested subscription to active.
	pub fn subscription_confirmation_received(&mut self, subscription_type: SubscriptionType) {
		let subscription_count = self.requested_subscriptions.remove(&subscription_type);

		// If we don't have a subscription count, we already got an unsubscription request and sent the unsubscribe.
		// In this case, we don't do anything here.
		if let Some(count) = subscription_count {
			self.active_subscriptions.insert(subscription_type, count);
		}
	}

	/// To be called when a subscription failure message is received from the server. Removes requested subscription.
	pub fn subscription_failure_received(&mut self, subscription_type: SubscriptionType) {
		self.requested_subscriptions.remove(&subscription_type);
	}
}
