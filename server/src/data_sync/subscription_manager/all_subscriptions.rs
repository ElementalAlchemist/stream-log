use super::one_subscription::SingleSubscriptionManager;
use super::user::UserSubscription;
use async_std::sync::{Arc, Mutex};
use async_std::task::block_on;
use futures::future::join_all;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use stream_log_shared::messages::subscriptions::{SubscriptionData, SubscriptionType};
use stream_log_shared::messages::user::UserData;
use tide_websockets::WebSocketConnection;

/// A manager for all the subscriptions we need to track
pub struct SubscriptionManager {
	event_subscriptions: HashMap<String, SingleSubscriptionManager>,
	user_subscriptions: HashMap<String, UserSubscription>,
}

impl SubscriptionManager {
	pub fn new() -> Self {
		Self {
			event_subscriptions: HashMap::new(),
			user_subscriptions: HashMap::new(),
		}
	}

	/// Shuts down the subscription manager and all subscription tasks.
	/// Assumes this is part of a full server shutdown, so this blocks on all the tasks ending.
	pub fn shutdown(&mut self) {
		let mut handles = Vec::with_capacity(self.event_subscriptions.len());
		for (_, subscription_manager) in self.event_subscriptions.drain() {
			handles.push(subscription_manager.thread_handle);
		}
		for handle in handles {
			block_on(handle);
		}
	}

	/// Subscribes the provided user with the provided associated connection to the provided event
	pub async fn subscribe_user_to_event(
		&mut self,
		event_id: &str,
		subscribing_user: &UserData,
		connection: Arc<Mutex<WebSocketConnection>>,
	) {
		match self.event_subscriptions.entry(event_id.to_owned()) {
			Entry::Occupied(mut event_subscription) => {
				event_subscription
					.get_mut()
					.subscribe_user(subscribing_user, connection)
					.await
			}
			Entry::Vacant(event_entry) => {
				let event_subscription =
					SingleSubscriptionManager::new(SubscriptionType::EventLogData(event_id.to_string()));
				event_subscription.subscribe_user(subscribing_user, connection).await;
				event_entry.insert(event_subscription);
			}
		}
	}

	/// Unsubscribes the provided user from the provided event
	pub async fn unsubscribe_user_from_event(&self, event_id: &str, user: &UserData) -> tide::Result<()> {
		if let Some(event_subscription) = self.event_subscriptions.get(event_id) {
			event_subscription.unsubscribe_user(user).await?;
		}
		Ok(())
	}

	/// Sends the given message to all subscribed users for the given event
	pub async fn broadcast_event_message(&mut self, event_id: &str, message: SubscriptionData) -> miette::Result<()> {
		if let Some(event_subscription) = self.event_subscriptions.get(event_id) {
			event_subscription.broadcast_message(message).await?;
		}
		Ok(())
	}

	/// Unsubscribes a user from all subscriptions
	pub async fn unsubscribe_user_from_all(&mut self, user: &UserData) {
		let mut futures = Vec::with_capacity(self.event_subscriptions.len());
		for event_subscription in self.event_subscriptions.values() {
			futures.push(event_subscription.unsubscribe_user(user));
		}
		self.user_subscriptions.remove(&user.id);
		join_all(futures).await;
	}

	pub fn add_user_subscription(&mut self, user: &UserData, connection: Arc<Mutex<WebSocketConnection>>) {
		self.user_subscriptions
			.insert(user.id.clone(), UserSubscription::new(connection));
	}
}
