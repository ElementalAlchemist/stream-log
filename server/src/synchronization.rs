use crate::models::Permission;
use async_std::channel::{unbounded, Sender};
use async_std::sync::{Arc, Mutex};
use async_std::task::{block_on, spawn, JoinHandle};
use futures::future::join_all;
use futures::StreamExt;
use miette::IntoDiagnostic;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::user::UserData;
use tide_websockets::WebSocketConnection;

/// A manager for all the subscriptions we need to track
pub struct SubscriptionManager {
	event_subscriptions: HashMap<String, EventSubscriptionManager>,
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
		permission: Permission,
		connection: Arc<Mutex<WebSocketConnection>>,
	) {
		match self.event_subscriptions.entry(event_id.to_owned()) {
			Entry::Occupied(mut event_subscription) => {
				event_subscription
					.get_mut()
					.subscribe_user(subscribing_user, connection, permission)
					.await
			}
			Entry::Vacant(event_entry) => {
				let event_subscription = EventSubscriptionManager::new();
				event_subscription
					.subscribe_user(subscribing_user, connection, permission)
					.await;
				event_entry.insert(event_subscription);
			}
		}
	}

	/// Unsubscribes the provided user from the provided event
	pub async fn unsubscribe_user_from_event(&self, event_id: &str, user: &UserData) {
		if let Some(event_subscription) = self.event_subscriptions.get(event_id) {
			event_subscription.unsubscribe_user(user).await;
		}
	}

	/// Gets the cached permission level for a user in a given event to which that user is subscribed.
	/// If the user is not subscribed to the event, returns None.
	pub async fn get_cached_user_permission(&self, event_id: &str, user: &UserData) -> Option<Permission> {
		match self.event_subscriptions.get(event_id) {
			Some(event_data) => event_data.get_cached_user_permission(user).await,
			None => None,
		}
	}

	/// Sends the given message to all subscribed users for the given event
	pub async fn broadcast_event_message(
		&mut self,
		event_id: &str,
		message: EventSubscriptionData,
	) -> miette::Result<()> {
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
			.insert(user.id.clone(), UserSubscription { connection });
	}
}

struct UserEventSubscriptionData {
	connection: Arc<Mutex<WebSocketConnection>>,
	permission: Permission,
}

/// Manages subscriptions for a single event
struct EventSubscriptionManager {
	thread_handle: JoinHandle<()>,
	subscription_send_channel: Sender<EventSubscriptionData>,
	subscriptions: Arc<Mutex<HashMap<String, UserEventSubscriptionData>>>,
}

impl EventSubscriptionManager {
	fn new() -> Self {
		let (broadcast_tx, mut broadcast_rx) = unbounded::<EventSubscriptionData>();
		let subscriptions: Arc<Mutex<HashMap<String, UserEventSubscriptionData>>> =
			Arc::new(Mutex::new(HashMap::new()));
		let thread_handle = spawn({
			let subscriptions = Arc::clone(&subscriptions);
			async move {
				while let Some(broadcast_msg) = broadcast_rx.next().await {
					let mut dead_connection_users: Vec<String> = Vec::new();
					let mut subscriptions = subscriptions.lock().await;
					for (user_id, user_subscription) in subscriptions.iter() {
						let stream = user_subscription.connection.lock().await;
						let send_result = stream.send_json(&broadcast_msg).await;
						if send_result.is_err() {
							dead_connection_users.push(user_id.clone());
						}
					}
					for user_id in dead_connection_users.iter() {
						subscriptions.remove(user_id);
					}
				}
			}
		});

		Self {
			thread_handle,
			subscription_send_channel: broadcast_tx,
			subscriptions,
		}
	}

	async fn subscribe_user(
		&self,
		user: &UserData,
		connection: Arc<Mutex<WebSocketConnection>>,
		permission: Permission,
	) {
		let mut subscriptions = self.subscriptions.lock().await;
		let user_subscription_data = UserEventSubscriptionData { connection, permission };
		subscriptions.insert(user.id.clone(), user_subscription_data);
	}

	async fn get_cached_user_permission(&self, user: &UserData) -> Option<Permission> {
		let subscriptions = self.subscriptions.lock().await;
		subscriptions
			.get(&user.id)
			.map(|subscription_data| subscription_data.permission)
	}

	async fn unsubscribe_user(&self, user: &UserData) {
		let mut subscriptions = self.subscriptions.lock().await;
		subscriptions.remove(&user.id);
	}

	async fn broadcast_message(&self, message: EventSubscriptionData) -> miette::Result<()> {
		self.subscription_send_channel.send(message).await.into_diagnostic()
	}
}

/// Manages the login subscription for a user
struct UserSubscription {
	connection: Arc<Mutex<WebSocketConnection>>,
}
