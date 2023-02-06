use crate::models::User;
use async_std::channel::{unbounded, Sender};
use async_std::sync::{Arc, Mutex};
use async_std::task::{spawn, JoinHandle};
use futures::StreamExt;
use miette::IntoDiagnostic;
use std::collections::HashMap;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use tide_websockets::WebSocketConnection;

pub struct SubscriptionManager {
	event_subscriptions: HashMap<String, EventSubscriptionManager>,
}

impl SubscriptionManager {
	pub fn new() -> Self {
		Self {
			event_subscriptions: HashMap::new(),
		}
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

struct EventSubscriptionManager {
	thread_handle: JoinHandle<()>,
	subscription_send_channel: Sender<EventSubscriptionData>,
	subscriptions: Arc<Mutex<HashMap<String, Arc<Mutex<WebSocketConnection>>>>>,
}

impl EventSubscriptionManager {
	fn new() -> Self {
		let (broadcast_tx, mut broadcast_rx) = unbounded::<EventSubscriptionData>();
		let subscriptions: Arc<Mutex<HashMap<String, Arc<Mutex<WebSocketConnection>>>>> =
			Arc::new(Mutex::new(HashMap::new()));
		let thread_handle = spawn({
			let subscriptions = Arc::clone(&subscriptions);
			async move {
				while let Some(broadcast_msg) = broadcast_rx.next().await {
					let mut dead_connection_users: Vec<String> = Vec::new();
					let mut subscriptions = subscriptions.lock().await;
					for (user_id, stream) in subscriptions.iter() {
						let stream = stream.lock().await;
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

	async fn broadcast_message(&self, message: EventSubscriptionData) -> miette::Result<()> {
		self.subscription_send_channel.send(message).await.into_diagnostic()
	}
}
