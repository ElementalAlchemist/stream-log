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
	user_subscriptions: HashMap<String, Sender<ConnectionUpdate>>,
	admin_user_subscriptions: SingleSubscriptionManager,
	admin_event_subscriptions: SingleSubscriptionManager,
	admin_permission_group_subscriptions: SingleSubscriptionManager,
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
			user_subscriptions: HashMap::new(),
			admin_user_subscriptions: SingleSubscriptionManager::new(SubscriptionType::AdminUsers),
			admin_event_subscriptions: SingleSubscriptionManager::new(SubscriptionType::AdminEvents),
			admin_permission_group_subscriptions: SingleSubscriptionManager::new(
				SubscriptionType::AdminPermissionGroups,
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

	/// Adds a user self-subscription for the provided user
	pub async fn subscribe_user_to_self(&mut self, user: &UserData, conn_update_tx: Sender<ConnectionUpdate>) {
		if self.user_subscriptions.contains_key(&user.id) {
			// We need to prevent any *other* data going to this connection so they don't get partial updates and think the connection is still alive.
			let _ = self.unsubscribe_user_from_all(user).await; // If there was an error, the unsubscription was still successful; it just couldn't be sent to the user.
		}
		self.user_subscriptions.insert(user.id.clone(), conn_update_tx);
	}

	/// Sends a message to a particular user
	pub async fn send_message_to_user(&mut self, user_id: &str, message: ConnectionUpdate) {
		let channel = self.user_subscriptions.get(user_id);
		if let Some(channel) = channel {
			let send_result = channel.send(message).await;
			if send_result.is_err() {
				channel.close(); // Be extra sure that the channel is closed
				self.user_subscriptions.remove(user_id);
			}
		}
	}

	/// Adds a user to the admin user list subscription
	pub async fn add_admin_user_subscription(&self, user: &UserData, update_channel: Sender<ConnectionUpdate>) {
		self.admin_user_subscriptions.subscribe_user(user, update_channel).await;
	}

	/// Removes a user from the admin user list subscription
	pub async fn remove_admin_user_subscription(&self, user: &UserData) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_user_subscriptions.unsubscribe_user(user).await
	}

	/// Sends the given message to all subscribed users for the admin user list
	pub async fn broadcast_admin_user_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_user_subscriptions.broadcast_message(message).await
	}

	/// Checks whether a user is subscribed to admin users
	pub async fn user_is_subscribed_to_admin_users(&self, user: &UserData) -> bool {
		self.admin_user_subscriptions.user_is_subscribed(user).await
	}

	/// Adds a user to the admin event list subscription
	pub async fn add_admin_event_subscription(&self, user: &UserData, update_channel: Sender<ConnectionUpdate>) {
		self.admin_event_subscriptions
			.subscribe_user(user, update_channel)
			.await;
	}

	/// Removes a user from the admin event list subscription
	pub async fn remove_admin_event_subscription(&self, user: &UserData) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_event_subscriptions.unsubscribe_user(user).await
	}

	/// Sends the given message to all subscribed users for the admin event list
	pub async fn broadcast_admin_event_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_event_subscriptions.broadcast_message(message).await
	}

	/// Checks whether a user is subscribed to admin events
	pub async fn user_is_subscribed_to_admin_events(&self, user: &UserData) -> bool {
		self.admin_event_subscriptions.user_is_subscribed(user).await
	}

	/// Adds a user to the admin permission group list subscription
	pub async fn add_admin_permission_group_subscription(
		&self,
		user: &UserData,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_permission_group_subscriptions
			.subscribe_user(user, update_channel)
			.await;
	}

	/// Removes a user from the admin permission group list subscription
	pub async fn remove_admin_permission_group_subscription(
		&self,
		user: &UserData,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_permission_group_subscriptions.unsubscribe_user(user).await
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

	/// Checks whether a user is subscribed to admin permission groups
	pub async fn user_is_subscribed_to_admin_permission_groups(&self, user: &UserData) -> bool {
		self.admin_permission_group_subscriptions.user_is_subscribed(user).await
	}

	/// Adds a user to the admin permission group user associations subscription
	pub async fn add_admin_permission_group_users_subscription(
		&self,
		user: &UserData,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_permission_group_user_subscriptions
			.subscribe_user(user, update_channel)
			.await;
	}

	/// Removes a user from the admin permission group user associations subscription
	pub async fn remove_admin_permission_group_users_subscription(
		&self,
		user: &UserData,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_permission_group_user_subscriptions
			.unsubscribe_user(user)
			.await
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

	/// Checks whether a user is subscribed to admin permission group user associations
	pub async fn user_is_subscribed_to_admin_permission_group_users(&self, user: &UserData) -> bool {
		self.admin_permission_group_user_subscriptions
			.user_is_subscribed(user)
			.await
	}

	/// Adds a user to the admin entry types subscription
	pub async fn add_admin_entry_types_subscription(&self, user: &UserData, update_channel: Sender<ConnectionUpdate>) {
		self.admin_entry_type_subscriptions
			.subscribe_user(user, update_channel)
			.await;
	}

	/// Removes a user from the admin entry types subscription
	pub async fn remove_admin_entry_types_subscription(
		&self,
		user: &UserData,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_entry_type_subscriptions.unsubscribe_user(user).await
	}

	/// Sends the given message to all subscribed users for admin entry types
	pub async fn broadcast_admin_entry_types_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_entry_type_subscriptions.broadcast_message(message).await
	}

	/// Checks whether a user is subscribed to admin entry types
	pub async fn user_is_subscribed_to_admin_entry_types(&self, user: &UserData) -> bool {
		self.admin_entry_type_subscriptions.user_is_subscribed(user).await
	}

	/// Adds a user to the admin entry types and event associations subscription
	pub async fn add_admin_entry_types_events_subscription(
		&self,
		user: &UserData,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_entry_type_event_subscriptions
			.subscribe_user(user, update_channel)
			.await;
	}

	/// Removes a user from the admin entry types and event associations subscription
	pub async fn remove_admin_entry_types_events_subscription(
		&self,
		user: &UserData,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_entry_type_event_subscriptions.unsubscribe_user(user).await
	}

	/// Sends the given message to all subscribed users for admin entry type and event associations
	pub async fn broadcast_admin_entry_types_events_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_entry_type_event_subscriptions
			.broadcast_message(message)
			.await
	}

	/// Checks whether a user is subscribed to admin entry type and event associations
	pub async fn user_is_subscribed_to_admin_entry_types_events(&self, user: &UserData) -> bool {
		self.admin_entry_type_event_subscriptions.user_is_subscribed(user).await
	}

	/// Adds a user to the admin tags subscription
	pub async fn add_admin_tags_subscription(&self, user: &UserData, update_channel: Sender<ConnectionUpdate>) {
		self.admin_tag_subscriptions.subscribe_user(user, update_channel).await;
	}

	/// Removes a user from the admin tags subscription
	pub async fn remove_admin_tags_subscription(&self, user: &UserData) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_tag_subscriptions.unsubscribe_user(user).await
	}

	/// Sends the given message to all subscribed users for admin tags
	pub async fn broadcast_admin_tags_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_tag_subscriptions.broadcast_message(message).await
	}

	/// Checks whether a user is subscribed to admin tags
	pub async fn user_is_subscribed_to_admin_tags(&self, user: &UserData) -> bool {
		self.admin_tag_subscriptions.user_is_subscribed(user).await
	}

	/// Adds a user to the admin event editors subscription
	pub async fn add_admin_editors_subscription(&self, user: &UserData, update_channel: Sender<ConnectionUpdate>) {
		self.admin_event_editor_subscriptions
			.subscribe_user(user, update_channel)
			.await;
	}

	/// Removes a user from the admin event editors subscription
	pub async fn remove_admin_editors_subscription(&self, user: &UserData) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_event_editor_subscriptions.unsubscribe_user(user).await
	}

	/// Sends the given message to all subscribed users for admin event editors
	pub async fn broadcast_admin_editors_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_event_editor_subscriptions.broadcast_message(message).await
	}

	/// Checks whether a user is subscribed to admin editors
	pub async fn user_is_subscribed_to_admin_editors(&self, user: &UserData) -> bool {
		self.admin_event_editor_subscriptions.user_is_subscribed(user).await
	}

	/// Unsubscribes a user from all subscriptions
	pub async fn unsubscribe_user_from_all(&mut self, user: &UserData) -> Result<(), SendError<ConnectionUpdate>> {
		self.user_subscriptions.remove(&user.id);
		let mut futures = Vec::with_capacity(self.event_subscriptions.len());
		for event_subscription in self.event_subscriptions.values() {
			futures.push(event_subscription.unsubscribe_user(user));
		}
		futures.push(self.admin_user_subscriptions.unsubscribe_user(user));
		futures.push(self.admin_event_subscriptions.unsubscribe_user(user));
		futures.push(self.admin_permission_group_subscriptions.unsubscribe_user(user));
		futures.push(self.admin_permission_group_user_subscriptions.unsubscribe_user(user));
		futures.push(self.admin_entry_type_subscriptions.unsubscribe_user(user));
		futures.push(self.admin_entry_type_event_subscriptions.unsubscribe_user(user));
		futures.push(self.admin_tag_subscriptions.unsubscribe_user(user));
		futures.push(self.admin_event_editor_subscriptions.unsubscribe_user(user));

		let results = join_all(futures).await;
		for result in results {
			result?;
		}
		Ok(())
	}
}
