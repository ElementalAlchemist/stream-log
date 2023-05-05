use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::stream::SplitStream;
use gloo_net::websocket::futures::WebSocket;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{
	EditorEventAssociation, EntryTypeEventAssociation, PermissionGroup, PermissionGroupEventAssociation,
	UserPermissionGroupAssociation,
};
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{InitialSubscriptionLoadData, SubscriptionData, SubscriptionType};
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::user_register::RegistrationResponse;
use stream_log_shared::messages::FromServerMessage;
use sycamore::prelude::*;

pub mod errors;
use errors::ErrorData;

pub mod event;
use event::EventSubscriptionSignals;

pub mod manager;
use manager::SubscriptionManager;

pub mod registration;
use registration::RegistrationData;

/// A struct containing all of the signals that can be updated by server messages.
#[derive(Clone)]
pub struct DataSignals {
	/// List of errors. These are displayed to the user.
	pub errors: RcSignal<Vec<ErrorData>>,

	/// Subscription data for each event for which we have a subscription.
	pub events: RcSignal<HashMap<String, EventSubscriptionSignals>>,

	/// When we're going through a registration workflow, contains all the data relevant for registering a new account.
	pub registration: RegistrationData,

	/// List of events available to the currently logged-in user.
	pub available_events: RcSignal<Vec<Event>>,

	/// List of all users registered.
	pub all_users: RcSignal<Vec<UserData>>,

	/// List of all events that exist.
	pub all_events: RcSignal<Vec<Event>>,

	/// List of all entry types that have been created.
	pub all_entry_types: RcSignal<Vec<EntryType>>,

	/// List of all permission groups that have been set up.
	pub all_permission_groups: RcSignal<Vec<PermissionGroup>>,

	/// List of associations between permission groups and events
	pub permission_group_event_associations: RcSignal<Vec<PermissionGroupEventAssociation>>,

	/// List of all tags that have been created.
	pub all_tags: RcSignal<Vec<Tag>>,

	/// List of all editor user/event pairings
	pub event_editors: RcSignal<Vec<EditorEventAssociation>>,

	/// List of all user/permission group pairings
	pub user_permission_groups: RcSignal<Vec<UserPermissionGroupAssociation>>,

	/// List of all pairings of entry types and events
	pub entry_type_event_associations: RcSignal<Vec<EntryTypeEventAssociation>>,
}

impl DataSignals {
	pub fn new() -> Self {
		Self {
			errors: create_rc_signal(Vec::new()),
			events: create_rc_signal(HashMap::new()),
			registration: RegistrationData::new(),
			available_events: create_rc_signal(Vec::new()),
			all_users: create_rc_signal(Vec::new()),
			all_events: create_rc_signal(Vec::new()),
			all_entry_types: create_rc_signal(Vec::new()),
			all_permission_groups: create_rc_signal(Vec::new()),
			permission_group_event_associations: create_rc_signal(Vec::new()),
			all_tags: create_rc_signal(Vec::new()),
			event_editors: create_rc_signal(Vec::new()),
			user_permission_groups: create_rc_signal(Vec::new()),
			entry_type_event_associations: create_rc_signal(Vec::new()),
		}
	}
}

/// The message update loop
pub async fn process_messages(ctx: Scope<'_>, mut ws_read: SplitStream<WebSocket>) {
	let data_signals: &DataSignals = use_context(ctx);
	let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);

	loop {
		let message: FromServerMessage = match read_websocket(&mut ws_read).await {
			Ok(msg) => msg,
			Err(_) => {
				data_signals.errors.modify().push(ErrorData::new(
					"The connection with the server has broken. If this wasn't expected, refresh the page.",
				));
				break;
			}
		};

		match message {
			FromServerMessage::InitialSubscriptionLoad(subscription_load_data) => {
				let mut subscription_manager = subscription_manager.lock().await;
				match *subscription_load_data {
					InitialSubscriptionLoadData::Event(
						event,
						permission_level,
						entry_types,
						tags,
						editors,
						event_log_entries,
					) => {
						let event_id = event.id.clone();
						let event = create_rc_signal(event);
						let permission = create_rc_signal(permission_level);
						let entry_types = create_rc_signal(entry_types);
						let tags = create_rc_signal(tags);
						let editors = create_rc_signal(editors);
						let event_log_entries = create_rc_signal(event_log_entries);

						let event_subscription_data = EventSubscriptionSignals {
							event,
							permission,
							entry_types,
							tags,
							editors,
							event_log_entries,
						};
						data_signals
							.events
							.modify()
							.insert(event_id.clone(), event_subscription_data);
						subscription_manager
							.subscription_confirmation_received(SubscriptionType::EventLogData(event_id));
					}
					InitialSubscriptionLoadData::AdminUsers(users) => {
						data_signals.all_users.set(users);
						subscription_manager.subscription_confirmation_received(SubscriptionType::AdminUsers);
					}
					InitialSubscriptionLoadData::AdminEvents(events) => {
						data_signals.all_events.set(events);
						subscription_manager.subscription_confirmation_received(SubscriptionType::AdminEvents);
					}
					InitialSubscriptionLoadData::AdminPermissionGroups(permission_groups, permission_group_events) => {
						data_signals.all_permission_groups.set(permission_groups);
						data_signals
							.permission_group_event_associations
							.set(permission_group_events);
						subscription_manager
							.subscription_confirmation_received(SubscriptionType::AdminPermissionGroups);
					}
					InitialSubscriptionLoadData::AdminPermissionGroupUsers(user_permission_groups) => {
						data_signals.user_permission_groups.set(user_permission_groups);
						subscription_manager
							.subscription_confirmation_received(SubscriptionType::AdminPermissionGroupUsers);
					}
					InitialSubscriptionLoadData::AdminEntryTypes(entry_types) => {
						data_signals.all_entry_types.set(entry_types);
						subscription_manager.subscription_confirmation_received(SubscriptionType::AdminEntryTypes);
					}
					InitialSubscriptionLoadData::AdminEntryTypesEvents(entry_types_events) => {
						data_signals.entry_type_event_associations.set(entry_types_events);
						subscription_manager
							.subscription_confirmation_received(SubscriptionType::AdminEntryTypesEvents);
					}
					InitialSubscriptionLoadData::AdminTags(tags) => {
						data_signals.all_tags.set(tags);
						subscription_manager.subscription_confirmation_received(SubscriptionType::AdminTags);
					}
					InitialSubscriptionLoadData::AdminEventEditors(event_editors) => {
						data_signals.event_editors.set(event_editors);
						subscription_manager.subscription_confirmation_received(SubscriptionType::AdminEventEditors);
					}
				}
			}
			FromServerMessage::SubscriptionMessage(subscription_data) => match *subscription_data {
				SubscriptionData::EventUpdate(event, update_data) => {
					let mut events_data = data_signals.events.modify();
					let Some(event_data) = events_data.get_mut(&event.id) else { continue; };
					match *update_data {
						EventSubscriptionData::UpdateEvent => event_data.event.set(event),
						EventSubscriptionData::NewLogEntry(log_entry) => {
							event_data.event_log_entries.modify().push(log_entry)
						}
						EventSubscriptionData::DeleteLogEntry(log_entry) => {
							let mut log_entries = event_data.event_log_entries.modify();
							let log_index = log_entries
								.iter()
								.enumerate()
								.find(|(_, entry)| log_entry.id == entry.id)
								.map(|(index, _)| index);
							if let Some(log_index) = log_index {
								log_entries.remove(log_index);
							}
						}
						EventSubscriptionData::UpdateLogEntry(log_entry) => {
							let mut log_entries = event_data.event_log_entries.modify();
							let existing_entry = log_entries.iter_mut().find(|entry| entry.id == log_entry.id);
							if let Some(entry) = existing_entry {
								*entry = log_entry;
							}
						}
						EventSubscriptionData::Typing(typing_data) => todo!(),
						EventSubscriptionData::NewTag(new_tag) => event_data.tags.modify().push(new_tag),
						EventSubscriptionData::DeleteTag(deleted_tag) => {
							let mut tags = event_data.tags.modify();
							let tag_index = tags
								.iter()
								.enumerate()
								.find(|(_, tag)| tag.id == deleted_tag.id)
								.map(|(index, _)| index);
							if let Some(index) = tag_index {
								tags.remove(index);
							}
						}
						EventSubscriptionData::AddEntryType(new_entry_type) => {
							event_data.entry_types.modify().push(new_entry_type)
						}
						EventSubscriptionData::UpdateEntryType(updated_entry_type) => {
							let mut entry_types = event_data.entry_types.modify();
							let entry_type = entry_types
								.iter_mut()
								.find(|entry_type| entry_type.id == updated_entry_type.id);
							if let Some(entry_type) = entry_type {
								*entry_type = updated_entry_type;
							}
						}
						EventSubscriptionData::DeleteEntryType(deleted_entry_type) => {
							let mut entry_types = event_data.entry_types.modify();
							let entry_type_index = entry_types
								.iter()
								.enumerate()
								.find(|(_, entry_type)| entry_type.id == deleted_entry_type.id)
								.map(|(index, _)| index);
							if let Some(index) = entry_type_index {
								entry_types.remove(index);
							}
						}
						EventSubscriptionData::AddEditor(new_editor) => event_data.editors.modify().push(new_editor),
						EventSubscriptionData::RemoveEditor(removed_editor) => {
							let mut editors = event_data.editors.modify();
							let editor_index = editors
								.iter()
								.enumerate()
								.find(|(_, editor)| editor.id == removed_editor.id)
								.map(|(index, _)| index);
							if let Some(index) = editor_index {
								editors.remove(index);
							}
						}
					}
				}
				SubscriptionData::UserUpdate(user_update) => todo!(),
				SubscriptionData::AdminEventsUpdate(event_data) => todo!(),
				SubscriptionData::AdminEntryTypesUpdate(entry_type_data) => todo!(),
				SubscriptionData::AdminEntryTypesEventsUpdate(entry_type_event_data) => todo!(),
				SubscriptionData::AdminPermissionGroupsUpdate(permission_group_update) => todo!(),
				SubscriptionData::AdminTagsUpdate(tag_data) => todo!(),
				SubscriptionData::AdminUsersUpdate(user_data) => todo!(),
				SubscriptionData::AdminEventEditorsUpdate(event_editor_data) => todo!(),
				SubscriptionData::AdminUserPermissionGroupsUpdate(user_permission_group_update) => todo!(),
			},
			FromServerMessage::Unsubscribed(subscription_type) => {
				todo!("Handle message and update subscription manager")
			}
			FromServerMessage::SubscriptionFailure(subscription_type, failure_info) => {
				todo!("Handle message and update subscription manager")
			}
			FromServerMessage::RegistrationResponse(response) => match response {
				RegistrationResponse::UsernameCheck(check_data) => {
					data_signals.registration.username_check.set(Some(check_data))
				}
				RegistrationResponse::Finalize(registration_data) => {
					data_signals.registration.final_register.set(Some(registration_data))
				}
			},
		}
	}
}
