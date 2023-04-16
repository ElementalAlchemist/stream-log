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
use stream_log_shared::messages::subscriptions::{SubscriptionData, SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::{UserData, UserSubscriptionUpdate};
use stream_log_shared::messages::FromServerMessage;
use tide_websockets::WebSocketConnection;

struct UserEventSubscriptionData {
	connection: Arc<Mutex<WebSocketConnection>>,
	permission: Permission,
}

/// Manages subscriptions for a single event
struct EventSubscriptionManager {
	event_id: String,
	thread_handle: JoinHandle<()>,
	subscription_send_channel: Sender<EventSubscriptionData>,
	subscriptions: Arc<Mutex<HashMap<String, UserEventSubscriptionData>>>,
}

impl EventSubscriptionManager {
	fn new(event_id: &str) -> Self {
		let (broadcast_tx, mut broadcast_rx) = unbounded::<EventSubscriptionData>();
		let subscriptions: Arc<Mutex<HashMap<String, UserEventSubscriptionData>>> =
			Arc::new(Mutex::new(HashMap::new()));
		let thread_handle = spawn({
			let subscriptions = Arc::clone(&subscriptions);
			async move {
				while let Some(broadcast_msg) = broadcast_rx.next().await {
					let mut subscriptions = subscriptions.lock().await;
					let mut dead_connection_users: Vec<String> = Vec::new();
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
			event_id: event_id.to_string(),
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

	async fn unsubscribe_user(&self, user: &UserData) -> tide::Result<()> {
		let mut subscriptions = self.subscriptions.lock().await;
		if let Some(user_sub_data) = subscriptions.remove(&user.id) {
			let connection = user_sub_data.connection.lock().await;
			let message = FromServerMessage::Unsubscribed(SubscriptionType::EventLogData(self.event_id.clone()));
			connection.send_json(&message).await?
		}
		Ok(())
	}

	async fn broadcast_message(&self, message: EventSubscriptionData) -> miette::Result<()> {
		self.subscription_send_channel.send(message).await.into_diagnostic()
	}
}
