use super::one_subscription::SingleSubscriptionManager;
use crate::data_sync::connection::ConnectionUpdate;
use crate::data_sync::UserDataUpdate;
use async_std::channel::{SendError, Sender};
use futures::future::join_all;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use stream_log_shared::messages::subscriptions::{SubscriptionData, SubscriptionType};
use stream_log_shared::messages::user::UserData;

/// A manager for all the subscriptions we need to track
pub struct SubscriptionManager {
	event_subscriptions: HashMap<String, SingleSubscriptionManager>,
	user_subscriptions: HashMap<String, HashMap<String, Sender<ConnectionUpdate>>>,
	admin_user_subscriptions: SingleSubscriptionManager,
	admin_event_subscriptions: SingleSubscriptionManager,
	admin_permission_group_subscriptions: SingleSubscriptionManager,
	admin_permission_group_user_subscriptions: SingleSubscriptionManager,
	admin_entry_type_subscriptions: SingleSubscriptionManager,
	admin_entry_type_event_subscriptions: SingleSubscriptionManager,
	admin_event_editor_subscriptions: SingleSubscriptionManager,
	admin_event_log_sections_subscriptions: SingleSubscriptionManager,
	admin_applications_subscriptions: SingleSubscriptionManager,
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
			admin_event_editor_subscriptions: SingleSubscriptionManager::new(SubscriptionType::AdminEventEditors),
			admin_event_log_sections_subscriptions: SingleSubscriptionManager::new(
				SubscriptionType::AdminEventLogSections,
			),
			admin_applications_subscriptions: SingleSubscriptionManager::new(SubscriptionType::AdminApplications),
		}
	}

	/// Shuts down the subscription manager and all subscription tasks.
	pub async fn shutdown(mut self) {
		let mut handles = Vec::new();
		for (_, subscription_manager) in self.event_subscriptions.drain() {
			handles.push(subscription_manager.thread_handle);
		}

		let subscription_shutdown_handles = vec![
			self.admin_user_subscriptions.shutdown(),
			self.admin_event_subscriptions.shutdown(),
			self.admin_permission_group_subscriptions.shutdown(),
			self.admin_permission_group_user_subscriptions.shutdown(),
			self.admin_entry_type_subscriptions.shutdown(),
			self.admin_entry_type_event_subscriptions.shutdown(),
			self.admin_event_editor_subscriptions.shutdown(),
			self.admin_applications_subscriptions.shutdown(),
		];
		for handle in join_all(subscription_shutdown_handles).await {
			handles.push(handle);
		}

		for (_, user_connection) in self.user_subscriptions.drain() {
			for (_, connection) in user_connection.iter() {
				connection.close();
			}
		}

		join_all(handles).await;
	}

	/// Subscribes the provided connection to the provided event
	pub async fn subscribe_to_event(
		&mut self,
		event_id: &str,
		connection_id: &str,
		conn_update_tx: Sender<ConnectionUpdate>,
	) {
		match self.event_subscriptions.entry(event_id.to_string()) {
			Entry::Occupied(mut event_subscription) => {
				event_subscription
					.get_mut()
					.subscribe(connection_id, conn_update_tx)
					.await
			}
			Entry::Vacant(event_entry) => {
				let event_subscription =
					SingleSubscriptionManager::new(SubscriptionType::EventLogData(event_id.to_string()));
				event_subscription.subscribe(connection_id, conn_update_tx).await;
				event_entry.insert(event_subscription);
			}
		}
	}

	/// Unsubscribes the provided connection from the provided event
	pub async fn unsubscribe_from_event(
		&self,
		event_id: &str,
		connection_id: &str,
	) -> Result<(), SendError<ConnectionUpdate>> {
		if let Some(event_subscription) = self.event_subscriptions.get(event_id) {
			event_subscription.unsubscribe(connection_id).await?;
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

	/// Adds a subscription to its associated user
	pub async fn subscribe_to_self_user(
		&mut self,
		connection_id: &str,
		user: &UserData,
		conn_update_tx: Sender<ConnectionUpdate>,
	) {
		match self.user_subscriptions.entry(user.id.clone()) {
			Entry::Occupied(mut user_subscription) => {
				user_subscription
					.get_mut()
					.insert(connection_id.to_owned(), conn_update_tx);
			}
			Entry::Vacant(user_entry) => {
				let mut user_subscriptions = HashMap::new();
				user_subscriptions.insert(connection_id.to_owned(), conn_update_tx);
				user_entry.insert(user_subscriptions);
			}
		}
	}

	/// Sends a user update to a particular user
	pub async fn send_message_to_user(&mut self, user_id: &str, message: UserDataUpdate) {
		let connections = self.user_subscriptions.get_mut(user_id);
		if let Some(connections) = connections {
			let mut dead_connection_ids: Vec<String> = Vec::new();
			for (connection_id, connection) in connections.iter() {
				let send_result = connection.send(ConnectionUpdate::UserUpdate(message.clone())).await;
				if send_result.is_err() {
					connection.close();
					dead_connection_ids.push(connection_id.clone());
				}
			}
			for connection_id in dead_connection_ids.iter() {
				connections.remove(connection_id);
			}
		}
	}

	/// Adds to the admin user list subscription
	pub async fn add_admin_user_subscription(&self, connection_id: &str, update_channel: Sender<ConnectionUpdate>) {
		self.admin_user_subscriptions
			.subscribe(connection_id, update_channel)
			.await;
	}

	/// Removes from the admin user list subscription
	pub async fn remove_admin_user_subscription(&self, connection_id: &str) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_user_subscriptions.unsubscribe(connection_id).await
	}

	/// Sends the given message to all subscribed connections for the admin user list
	pub async fn broadcast_admin_user_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_user_subscriptions.broadcast_message(message).await
	}

	/// Checks whether a connection is subscribed to admin users
	pub async fn is_subscribed_to_admin_users(&self, connection_id: &str) -> bool {
		self.admin_user_subscriptions.is_subscribed(connection_id).await
	}

	/// Adds to the admin event list subscription
	pub async fn add_admin_event_subscription(&self, connection_id: &str, update_channel: Sender<ConnectionUpdate>) {
		self.admin_event_subscriptions
			.subscribe(connection_id, update_channel)
			.await;
	}

	/// Removes from the admin event list subscription
	pub async fn remove_admin_event_subscription(
		&self,
		connection_id: &str,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_event_subscriptions.unsubscribe(connection_id).await
	}

	/// Sends the given message to all subscribed connections for the admin event list
	pub async fn broadcast_admin_event_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_event_subscriptions.broadcast_message(message).await
	}

	/// Checks whether a connection is subscribed to admin events
	pub async fn is_subscribed_to_admin_events(&self, connection_id: &str) -> bool {
		self.admin_event_subscriptions.is_subscribed(connection_id).await
	}

	/// Adds to the admin permission group list subscription
	pub async fn add_admin_permission_group_subscription(
		&self,
		connection_id: &str,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_permission_group_subscriptions
			.subscribe(connection_id, update_channel)
			.await;
	}

	/// Removes from the admin permission group list subscription
	pub async fn remove_admin_permission_group_subscription(
		&self,
		connection_id: &str,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_permission_group_subscriptions
			.unsubscribe(connection_id)
			.await
	}

	/// Sends the given message to all subscribed connections for the admin permission group list
	pub async fn broadcast_admin_permission_groups_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_permission_group_subscriptions
			.broadcast_message(message)
			.await
	}

	/// Checks whether a connection is subscribed to admin permission groups
	pub async fn is_subscribed_to_admin_permission_groups(&self, connection_id: &str) -> bool {
		self.admin_permission_group_subscriptions
			.is_subscribed(connection_id)
			.await
	}

	/// Adds to the admin permission group user associations subscription
	pub async fn add_admin_permission_group_users_subscription(
		&self,
		connection_id: &str,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_permission_group_user_subscriptions
			.subscribe(connection_id, update_channel)
			.await;
	}

	/// Removes from the admin permission group user associations subscription
	pub async fn remove_admin_permission_group_users_subscription(
		&self,
		connection_id: &str,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_permission_group_user_subscriptions
			.unsubscribe(connection_id)
			.await
	}

	/// Sends the given message to all subscribed connections for admin permission group user associations
	pub async fn broadcast_admin_permission_group_users_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_permission_group_user_subscriptions
			.broadcast_message(message)
			.await
	}

	/// Checks whether a connection is subscribed to admin permission group user associations
	pub async fn is_subscribed_to_admin_permission_group_users(&self, connection_id: &str) -> bool {
		self.admin_permission_group_user_subscriptions
			.is_subscribed(connection_id)
			.await
	}

	/// Adds to the admin entry types subscription
	pub async fn add_admin_entry_types_subscription(
		&self,
		connection_id: &str,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_entry_type_subscriptions
			.subscribe(connection_id, update_channel)
			.await;
	}

	/// Removes from the admin entry types subscription
	pub async fn remove_admin_entry_types_subscription(
		&self,
		connection_id: &str,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_entry_type_subscriptions.unsubscribe(connection_id).await
	}

	/// Sends the given message to all subscribed connections for admin entry types
	pub async fn broadcast_admin_entry_types_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_entry_type_subscriptions.broadcast_message(message).await
	}

	/// Checks whether a connection is subscribed to admin entry types
	pub async fn is_subscribed_to_admin_entry_types(&self, connection_id: &str) -> bool {
		self.admin_entry_type_subscriptions.is_subscribed(connection_id).await
	}

	/// Adds to the admin entry types and event associations subscription
	pub async fn add_admin_entry_types_events_subscription(
		&self,
		connection_id: &str,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_entry_type_event_subscriptions
			.subscribe(connection_id, update_channel)
			.await;
	}

	/// Removes from the admin entry types and event associations subscription
	pub async fn remove_admin_entry_types_events_subscription(
		&self,
		connection_id: &str,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_entry_type_event_subscriptions
			.unsubscribe(connection_id)
			.await
	}

	/// Sends the given message to all subscribed connections for admin entry type and event associations
	pub async fn broadcast_admin_entry_types_events_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_entry_type_event_subscriptions
			.broadcast_message(message)
			.await
	}

	/// Checks whether a connection is subscribed to admin entry type and event associations
	pub async fn is_subscribed_to_admin_entry_types_events(&self, connection_id: &str) -> bool {
		self.admin_entry_type_event_subscriptions
			.is_subscribed(connection_id)
			.await
	}

	/// Adds to the admin event editors subscription
	pub async fn add_admin_editors_subscription(&self, connection_id: &str, update_channel: Sender<ConnectionUpdate>) {
		self.admin_event_editor_subscriptions
			.subscribe(connection_id, update_channel)
			.await;
	}

	/// Removes from the admin event editors subscription
	pub async fn remove_admin_editors_subscription(
		&self,
		connection_id: &str,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_event_editor_subscriptions.unsubscribe(connection_id).await
	}

	/// Sends the given message to all subscribed connections for admin event editors
	pub async fn broadcast_admin_editors_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_event_editor_subscriptions.broadcast_message(message).await
	}

	/// Checks whether a connection is subscribed to admin editors
	pub async fn is_subscribed_to_admin_editors(&self, connection_id: &str) -> bool {
		self.admin_event_editor_subscriptions.is_subscribed(connection_id).await
	}

	/// Adds to the admin event log sections subscription
	pub async fn add_admin_event_log_sections_subscription(
		&self,
		connection_id: &str,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_event_log_sections_subscriptions
			.subscribe(connection_id, update_channel)
			.await;
	}

	/// Removes from the admin event log sections subscription
	pub async fn remove_admin_event_log_sections_subscription(
		&self,
		connection_id: &str,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_event_log_sections_subscriptions
			.unsubscribe(connection_id)
			.await
	}

	/// Sends the given message to all subscribed connections for admin event log sections
	pub async fn broadcast_admin_event_log_sections_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_event_log_sections_subscriptions
			.broadcast_message(message)
			.await
	}

	/// Checks whether a connection is subscribed to admin event log sections
	pub async fn is_subscribed_to_admin_event_log_sections(&self, connection_id: &str) -> bool {
		self.admin_event_log_sections_subscriptions
			.is_subscribed(connection_id)
			.await
	}

	/// Adds to the admin applications subscription
	pub async fn add_admin_applications_subscription(
		&self,
		connection_id: &str,
		update_channel: Sender<ConnectionUpdate>,
	) {
		self.admin_applications_subscriptions
			.subscribe(connection_id, update_channel)
			.await;
	}

	/// Removes from the admin applications subscription
	pub async fn remove_admin_applications_subscription(
		&self,
		connection_id: &str,
	) -> Result<(), SendError<ConnectionUpdate>> {
		self.admin_applications_subscriptions.unsubscribe(connection_id).await
	}

	/// Sends the given message to all subscribed connections for admin applications
	pub async fn broadcast_admin_applications_message(
		&self,
		message: SubscriptionData,
	) -> Result<(), SendError<SubscriptionData>> {
		self.admin_applications_subscriptions.broadcast_message(message).await
	}

	/// Checks whether a connection is subscribed to admin applications
	pub async fn is_subscribed_to_admin_applications(&self, connection_id: &str) -> bool {
		self.admin_applications_subscriptions.is_subscribed(connection_id).await
	}

	/// Unsubscribes a connection from all subscriptions
	pub async fn unsubscribe_from_all(&mut self, connection_id: &str) -> Result<(), SendError<ConnectionUpdate>> {
		let mut futures = Vec::with_capacity(self.event_subscriptions.len());
		for event_subscription in self.event_subscriptions.values() {
			futures.push(event_subscription.unsubscribe(connection_id));
		}
		for user_subscription in self.user_subscriptions.values_mut() {
			user_subscription.remove(connection_id);
		}
		futures.push(self.admin_user_subscriptions.unsubscribe(connection_id));
		futures.push(self.admin_event_subscriptions.unsubscribe(connection_id));
		futures.push(self.admin_permission_group_subscriptions.unsubscribe(connection_id));
		futures.push(
			self.admin_permission_group_user_subscriptions
				.unsubscribe(connection_id),
		);
		futures.push(self.admin_entry_type_subscriptions.unsubscribe(connection_id));
		futures.push(self.admin_entry_type_event_subscriptions.unsubscribe(connection_id));
		futures.push(self.admin_event_editor_subscriptions.unsubscribe(connection_id));
		futures.push(self.admin_applications_subscriptions.unsubscribe(connection_id));

		let results = join_all(futures).await;
		for result in results {
			result?;
		}
		Ok(())
	}
}
