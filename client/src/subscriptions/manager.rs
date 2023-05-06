use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::{Message, WebSocketError};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
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

impl fmt::Display for SubscriptionError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::SerializeError(error) => write!(f, "An error occurred during serialization: {}", error),
			Self::SendError(error) => write!(f, "An error occurred when sending: {}", error),
		}
	}
}

#[derive(Debug, Default)]
pub struct SubscriptionManager {
	active_subscriptions: HashMap<SubscriptionType, u32>,
	requested_subscriptions: HashMap<SubscriptionType, u32>,
}

impl SubscriptionManager {
	/// Removes a subscription for data.
	pub fn remove_subscription(&mut self, subscription_type: SubscriptionType) {
		if let Entry::Occupied(mut active_entry) = self.active_subscriptions.entry(subscription_type.clone()) {
			let current_count = *active_entry.get() - 1;
			if current_count == 0 {
				active_entry.remove();
				true
			} else {
				*active_entry.get_mut() = current_count;
				false
			}
		} else if let Entry::Occupied(mut requested_entry) = self.requested_subscriptions.entry(subscription_type) {
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
	}

	/// Changes the current set of subscriptions so that it contains only the one specified subscription type.
	pub async fn set_subscription(
		&mut self,
		subscription_type: SubscriptionType,
		stream: &mut SplitSink<WebSocket, Message>,
	) -> Result<(), SubscriptionError> {
		let mut new_active_subscriptions: HashMap<SubscriptionType, u32> = HashMap::new();
		let mut new_requested_subscriptions: HashMap<SubscriptionType, u32> = HashMap::new();
		let mut unsubscription_messages: Vec<Message> = Vec::new();
		for (current_subscription, _) in self.active_subscriptions.iter() {
			if *current_subscription == subscription_type {
				new_active_subscriptions.insert(current_subscription.clone(), 1);
			} else {
				let unsubscription_message = FromClientMessage::EndSubscription(current_subscription.clone());
				let unsubscription_message_json = serde_json::to_string(&unsubscription_message)?;
				unsubscription_messages.push(Message::Text(unsubscription_message_json));
			}
		}
		for (current_subscription, _) in self.requested_subscriptions.iter() {
			if *current_subscription == subscription_type {
				new_requested_subscriptions.insert(current_subscription.clone(), 1);
			} else {
				let unsubscription_message = FromClientMessage::EndSubscription(current_subscription.clone());
				let unsubscription_message_json = serde_json::to_string(&unsubscription_message)?;
				unsubscription_messages.push(Message::Text(unsubscription_message_json));
			}
		}

		for message in unsubscription_messages {
			stream.feed(message).await?;
		}
		stream.flush().await?;

		self.active_subscriptions = new_active_subscriptions;
		self.requested_subscriptions = new_requested_subscriptions;

		if self.active_subscriptions.is_empty() && self.requested_subscriptions.is_empty() {
			let subscription_message = FromClientMessage::StartSubscription(subscription_type.clone());
			let subscription_message_json = serde_json::to_string(&subscription_message)?;
			stream.send(Message::Text(subscription_message_json)).await?;
			self.requested_subscriptions.insert(subscription_type, 1);
		}

		Ok(())
	}

	/// Changes the current set of subscriptions to match a specific set of subscriptions
	pub async fn set_subscriptions(
		&mut self,
		subscription_types: Vec<SubscriptionType>,
		stream: &mut SplitSink<WebSocket, Message>,
	) -> Result<(), SubscriptionError> {
		let mut subscription_update_messages: Vec<Message> = Vec::new();
		let mut new_subscriptions: HashMap<SubscriptionType, u32> = HashMap::new();
		for subscription in subscription_types {
			*new_subscriptions.entry(subscription).or_default() += 1;
		}

		let mut new_active_subscriptions = HashMap::new();
		for current_subscription in self.active_subscriptions.keys() {
			if let Some(new_count) = new_subscriptions.remove(current_subscription) {
				new_active_subscriptions.insert(current_subscription.clone(), new_count);
			} else {
				let unsubscription_message = FromClientMessage::EndSubscription(current_subscription.clone());
				let unsubscription_message_json = serde_json::to_string(&unsubscription_message)?;
				subscription_update_messages.push(Message::Text(unsubscription_message_json));
			}
		}

		let mut new_requested_subscriptions = HashMap::new();
		for current_subscription in self.requested_subscriptions.keys() {
			if let Some(new_count) = new_subscriptions.remove(current_subscription) {
				new_requested_subscriptions.insert(current_subscription.clone(), new_count);
			} else {
				let unsubscription_message = FromClientMessage::EndSubscription(current_subscription.clone());
				let unsubscription_message_json = serde_json::to_string(&unsubscription_message)?;
				subscription_update_messages.push(Message::Text(unsubscription_message_json));
			}
		}

		for (new_subscription, new_count) in new_subscriptions.drain() {
			let subscription_message = FromClientMessage::StartSubscription(new_subscription.clone());
			let subscription_message_json = serde_json::to_string(&subscription_message)?;
			subscription_update_messages.push(Message::Text(subscription_message_json));
			new_requested_subscriptions.insert(new_subscription, new_count);
		}

		for message in subscription_update_messages {
			stream.feed(message).await?;
		}
		stream.flush().await?;

		self.active_subscriptions = new_active_subscriptions;
		self.requested_subscriptions = new_requested_subscriptions;

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
