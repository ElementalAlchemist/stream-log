use super::one_subscription::SingleSubscriptionManager;
use crate::data_sync::connection::ConnectionUpdate;
use async_std::channel::{SendError, Sender};
use async_std::task::block_on;
use futures::future::join_all;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use stream_log_shared::messages::subscriptions::{SubscriptionData, SubscriptionType};
use stream_log_shared::messages::user::UserData;

/// A manager for all the subscriptions we need to track
pub struct SubscriptionManager {
	event_subscriptions: HashMap<String, SingleSubscriptionManager>,
	admin_user_subscriptions: SingleSubscriptionManager,
	admin_event_subscriptions: SingleSubscriptionManager,
	admin_permission_group_subscriptions: SingleSubscriptionManager,
	admin_permission_group_event_subscriptions: SingleSubscriptionManager,
	admin_permission_group_user_subscriptions: SingleSubscriptionManager,
	admin_entry_type_subscriptions: SingleSubscriptionManager,
	admin_entry_type_event_subscriptions: SingleSubscriptionManager,
	admin_tag_subscriptions: SingleSubscriptionManager,
	admin_event_editor_subscriptions: SingleSubscriptionManager,
}

impl SubscriptionManager {
	pub fn new() -> Self {
		Self {
			event_subscriptions: HashMap::new(),
			admin_user_subscriptions: SingleSubscriptionManager::new(SubscriptionType::AdminUsers),
			admin_event_subscriptions: SingleSubscriptionManager::new(SubscriptionType::AdminEvents),
			admin_permission_group_subscriptions: SingleSubscriptionManager::new(
				SubscriptionType::AdminPermissionGroups,
			),
			admin_permission_group_event_subscriptions: SingleSubscriptionManager::new(
				SubscriptionType::AdminPermissionGroupEvents,
			),
			admin_permission_group_user_subscriptions: SingleSubscriptionManager::new(
				SubscriptionType::AdminPermissionGroupUsers,
			),
			admin_entry_type_subscriptions: SingleSubscriptionManager::new(SubscriptionType::AdminEntryTypes),
			admin_entry_type_event_subscriptions: SingleSubscriptionManager::new(
				SubscriptionType::AdminEntryTypesEvents,
			),
			admin_tag_subscriptions: SingleSubscriptionManager::new(SubscriptionType::AdminTags),
			admin_event_editor_subscriptions: SingleSubscriptionManager::new(SubscriptionType::AdminEventEditors),
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
		conn_update_tx: Sender<ConnectionUpdate>,
	) {
		match self.event_subscriptions.entry(event_id.to_owned()) {
			Entry::Occupied(mut event_subscription) => {
				event_subscription
					.get_mut()
					.subscribe_user(subscribing_user, conn_update_tx)
					.await
			}
			Entry::Vacant(event_entry) => {
				let event_subscription =
					SingleSubscriptionManager::new(SubscriptionType::EventLogData(event_id.to_string()));
				event_subscription
					.subscribe_user(subscribing_user, conn_update_tx)
					.await;
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
	pub async fn broadcast_event_message(
		&self,
		event_id: &str,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
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
		join_all(futures).await;
	}

	/// Adds a user to the admin user list subscription
	pub async fn add_admin_user_subscription(&mut self, user: &UserData, update_channel: Sender<ConnectionUpdate>) {
		self.admin_user_subscriptions.subscribe_user(user, update_channel).await;
	}

	/// Sends the given message to all subscribed users for the admin user list
	pub async fn broadcast_admin_user_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_user_subscriptions.broadcast_message(message).await
	}

	/// Adds a user to the admin event list subscription
	pub async fn add_admin_event_subscription(&mut self, user: &UserData, update_channel: Sender<ConnectionUpdate>) {
		self.admin_event_subscriptions
			.subscribe_user(user, update_channel)
			.await;
	}

	/// Sends the given message to all subscribed users for the admin event list
	pub async fn broadcast_admin_event_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_event_subscriptions.broadcast_message(message).await
	}

	/// Adds a user to the admin permission group list subscription
	pub async fn add_admin_permission_group_subscription(
		&mut self,
		user: &UserData,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_permission_group_subscriptions
			.subscribe_user(user, update_channel)
			.await;
	}

	/// Sends the given message to all subscribed users for the admin permission group list
	pub async fn broadcast_admin_permission_groups_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_permission_group_subscriptions
			.broadcast_message(message)
			.await
	}

	/// Adds a user to the admin permission group event associations subscription
	pub async fn add_admin_permission_group_events_subscription(
		&mut self,
		user: &UserData,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_permission_group_event_subscriptions
			.subscribe_user(user, update_channel)
			.await;
	}

	/// Sends the given message to all subscribed users for admin permission group event associations
	pub async fn broadcast_admin_permission_group_events_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_permission_group_event_subscriptions
			.broadcast_message(message)
			.await
	}

	/// Adds a user to the admin permission group user associations subscription
	pub async fn add_admin_permission_group_users_subscription(
		&mut self,
		user: &UserData,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_permission_group_user_subscriptions
			.subscribe_user(user, update_channel)
			.await;
	}

	/// Sends the given message to all subscribed users for admin permission group user associations
	pub async fn broadcast_admin_permission_group_users_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_permission_group_user_subscriptions
			.broadcast_message(message)
			.await
	}
}
