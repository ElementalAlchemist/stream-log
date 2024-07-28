// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::data_sync::connection::ConnectionUpdate;
use async_std::channel::{unbounded, SendError, Sender};
use async_std::stream::StreamExt;
use async_std::sync::{Arc, Mutex};
use async_std::task::{spawn, JoinHandle};
use std::collections::HashMap;
use stream_log_shared::messages::subscriptions::{SubscriptionData, SubscriptionType};
use stream_log_shared::messages::FromServerMessage;

/// Manages subscriptions for a single set of subscription events
pub struct SingleSubscriptionManager {
	subscription_type: SubscriptionType,
	pub thread_handle: JoinHandle<()>,
	subscription_send_channel: Sender<SubscriptionData>,
	subscriptions: Arc<Mutex<HashMap<String, SingleSubscriptionData>>>,
}

impl SingleSubscriptionManager {
	pub fn new(subscription_type: SubscriptionType) -> Self {
		let (broadcast_tx, mut broadcast_rx) = unbounded::<SubscriptionData>();
		let subscriptions: Arc<Mutex<HashMap<String, SingleSubscriptionData>>> = Arc::new(Mutex::new(HashMap::new()));
		let thread_handle = spawn({
			let subscriptions = Arc::clone(&subscriptions);
			async move {
				while let Some(broadcast_msg) = broadcast_rx.next().await {
					let mut subscriptions = subscriptions.lock().await;
					let mut dead_connection_users: Vec<String> = Vec::new();
					for (user_id, user_subscription) in subscriptions.iter() {
						let message = FromServerMessage::SubscriptionMessage(Box::new(broadcast_msg.clone()));
						let send_result = user_subscription
							.channel
							.send(ConnectionUpdate::SendData(Box::new(message)))
							.await;
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
			subscription_type,
			thread_handle,
			subscription_send_channel: broadcast_tx,
			subscriptions,
		}
	}

	pub async fn subscribe(&self, connection_id: &str, channel: Sender<ConnectionUpdate>) {
		let mut subscriptions = self.subscriptions.lock().await;
		let subscription_data = SingleSubscriptionData { channel };
		subscriptions.insert(connection_id.to_owned(), subscription_data);
	}

	/// Unsubscribes a connection. Returns a result indicating whether the connection was notified successfully.
	pub async fn unsubscribe(&self, connection_id: &str) -> Result<(), SendError<ConnectionUpdate>> {
		let mut subscriptions = self.subscriptions.lock().await;
		if let Some(subscription_data) = subscriptions.remove(connection_id) {
			let message = FromServerMessage::Unsubscribed(self.subscription_type.clone());
			let message = ConnectionUpdate::SendData(Box::new(message));
			subscription_data.channel.send(message).await?;
		}
		Ok(())
	}

	pub async fn is_subscribed(&self, connection_id: &str) -> bool {
		self.subscriptions.lock().await.contains_key(connection_id)
	}

	pub async fn broadcast_message(&self, message: SubscriptionData) -> Result<(), SendError<SubscriptionData>> {
		self.subscription_send_channel.send(message).await
	}

	pub async fn shutdown(self) -> JoinHandle<()> {
		let mut subscriptions = self.subscriptions.lock().await;
		let mut send_futures = Vec::new();
		for (_, subscription_data) in subscriptions.drain() {
			let message = ConnectionUpdate::SendData(Box::new(FromServerMessage::Unsubscribed(
				self.subscription_type.clone(),
			)));
			send_futures.push(subscription_data.channel.send(message).await);
		}
		self.thread_handle
	}
}

struct SingleSubscriptionData {
	channel: Sender<ConnectionUpdate>,
}
