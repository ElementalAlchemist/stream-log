use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::stream::SplitStream;
use gloo_net::websocket::futures::WebSocket;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{
	AdminEntryTypeData, AdminEntryTypeEventData, AdminEventData, AdminEventEditorData, AdminPermissionGroupData,
	AdminTagData, AdminUserPermissionGroupData, EditorEventAssociation, EntryTypeEventAssociation, PermissionGroup,
	PermissionGroupEventAssociation, UserPermissionGroupAssociation,
};
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_subscription::EventSubscriptionData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::tags::{AvailableTagData, Tag};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::user_register::RegistrationResponse;
use stream_log_shared::messages::{DataError, FromServerMessage};
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

	/// List of tags available to be entered in the event log.
	pub available_tags: RcSignal<Vec<Tag>>,

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
			available_tags: create_rc_signal(Vec::new()),
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
						editors,
						event_log_entries,
					) => {
						let event_id = event.id.clone();
						let event = create_rc_signal(event);
						let permission = create_rc_signal(permission_level);
						let entry_types = create_rc_signal(entry_types);
						let editors = create_rc_signal(editors);
						let event_log_entries = create_rc_signal(event_log_entries);

						let event_subscription_data = EventSubscriptionSignals {
							event,
							permission,
							entry_types,
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
					InitialSubscriptionLoadData::AvailableTags(tags) => {
						data_signals.available_tags.set(tags);
						subscription_manager.subscription_confirmation_received(SubscriptionType::AvailableTags);
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
				SubscriptionData::UserUpdate(user_update) => {
					let user_signal: &Signal<Option<UserData>> = use_context(ctx);
					user_signal.set(Some(user_update.user));
					data_signals.available_events.set(user_update.available_events);
				}
				SubscriptionData::AvailableTagsUpdate(tag_update) => match tag_update {
					AvailableTagData::UpdateTag(updated_tag) => {
						let mut available_tags = data_signals.available_tags.modify();
						let existing_tag = available_tags.iter_mut().find(|tag| tag.id == updated_tag.id);
						match existing_tag {
							Some(tag) => *tag = updated_tag,
							None => available_tags.push(updated_tag),
						}
					}
					AvailableTagData::RemoveTag(removed_tag) => {
						let mut available_tags = data_signals.available_tags.modify();
						let tag_index = available_tags
							.iter()
							.enumerate()
							.find(|(_, tag)| tag.id == removed_tag.id)
							.map(|(index, _)| index);
						if let Some(index) = tag_index {
							available_tags.remove(index);
						}
					}
				},
				SubscriptionData::AdminEventsUpdate(event_data) => match event_data {
					AdminEventData::UpdateEvent(event) => {
						let mut all_events = data_signals.all_events.modify();
						let event_data = all_events.iter_mut().find(|an_event| an_event.id == event.id);
						match event_data {
							Some(event_data) => *event_data = event,
							None => all_events.push(event),
						}
					}
				},
				SubscriptionData::AdminEntryTypesUpdate(entry_type_data) => match entry_type_data {
					AdminEntryTypeData::UpdateEntryType(entry_type) => {
						let mut all_entry_types = data_signals.all_entry_types.modify();
						let entry_type_data = all_entry_types.iter_mut().find(|et| et.id == entry_type.id);
						match entry_type_data {
							Some(entry_type_data) => *entry_type_data = entry_type,
							None => all_entry_types.push(entry_type),
						}
					}
				},
				SubscriptionData::AdminEntryTypesEventsUpdate(entry_type_event_data) => match entry_type_event_data {
					AdminEntryTypeEventData::AddTypeToEvent(entry_type_event_association) => {
						let mut entry_type_event_associations = data_signals.entry_type_event_associations.modify();
						let exists = entry_type_event_associations.iter().any(|association| {
							association.entry_type.id == entry_type_event_association.entry_type.id
								&& association.event.id == entry_type_event_association.event.id
						});
						if !exists {
							entry_type_event_associations.push(entry_type_event_association);
						}
					}
					AdminEntryTypeEventData::RemoveTypeFromEvent(entry_type_event_association) => {
						let mut entry_type_event_associations = data_signals.entry_type_event_associations.modify();
						let association_index = entry_type_event_associations
							.iter()
							.enumerate()
							.find(|(_, association)| {
								association.entry_type.id == entry_type_event_association.entry_type.id
									&& association.event.id == entry_type_event_association.event.id
							})
							.map(|(index, _)| index);
						if let Some(index) = association_index {
							entry_type_event_associations.remove(index);
						}
					}
				},
				SubscriptionData::AdminPermissionGroupsUpdate(permission_group_update) => match permission_group_update
				{
					AdminPermissionGroupData::UpdateGroup(permission_group) => {
						let mut permission_groups = data_signals.all_permission_groups.modify();
						let existing_group = permission_groups
							.iter_mut()
							.find(|group| group.id == permission_group.id);
						match existing_group {
							Some(group) => *group = permission_group,
							None => permission_groups.push(permission_group),
						}
					}
					AdminPermissionGroupData::SetEventPermissionForGroup(permission_group_event_association) => {
						let mut permission_group_event_associations =
							data_signals.permission_group_event_associations.modify();
						let association = permission_group_event_associations.iter_mut().find(|association| {
							association.group == permission_group_event_association.group
								&& association.event == permission_group_event_association.event
						});
						match association {
							Some(association) => *association = permission_group_event_association,
							None => permission_group_event_associations.push(permission_group_event_association),
						}
					}
					AdminPermissionGroupData::RemoveEventFromGroup(group, event) => {
						let mut permission_group_event_associations =
							data_signals.permission_group_event_associations.modify();
						let association_index = permission_group_event_associations
							.iter()
							.enumerate()
							.find(|(_, association)| association.group == group.id && association.event == event.id)
							.map(|(index, _)| index);
						if let Some(index) = association_index {
							permission_group_event_associations.remove(index);
						}
					}
				},
				SubscriptionData::AdminTagsUpdate(tag_data) => match tag_data {
					AdminTagData::UpdateTag(tag) => {
						let mut all_tags = data_signals.all_tags.modify();
						let existing_tag = all_tags.iter_mut().find(|check_tag| check_tag.id == tag.id);
						match existing_tag {
							Some(existing_tag) => *existing_tag = tag,
							None => all_tags.push(tag),
						}
					}
					AdminTagData::RemoveTag(tag) => {
						let mut all_tags = data_signals.all_tags.modify();
						let tag_index = all_tags
							.iter()
							.enumerate()
							.find(|(_, check_tag)| check_tag.id == tag.id)
							.map(|(index, _)| index);
						if let Some(index) = tag_index {
							all_tags.remove(index);
						}
					}
				},
				SubscriptionData::AdminUsersUpdate(user_data) => {
					let mut all_users = data_signals.all_users.modify();
					let existing_user = all_users.iter_mut().find(|user| user.id == user_data.id);
					match existing_user {
						Some(user) => *user = user_data,
						None => all_users.push(user_data),
					}
				}
				SubscriptionData::AdminEventEditorsUpdate(event_editor_data) => match event_editor_data {
					AdminEventEditorData::AddEditor(editor_event_association) => {
						let mut event_editors = data_signals.event_editors.modify();
						if !event_editors.iter().any(|association| {
							association.editor.id == editor_event_association.editor.id
								&& association.event.id == editor_event_association.event.id
						}) {
							event_editors.push(editor_event_association);
						}
					}
					AdminEventEditorData::RemoveEditor(editor_event_association) => {
						let mut event_editors = data_signals.event_editors.modify();
						let association_index = event_editors
							.iter()
							.enumerate()
							.find(|(_, association)| {
								association.editor.id == editor_event_association.editor.id
									&& association.event.id == editor_event_association.event.id
							})
							.map(|(index, _)| index);
						if let Some(index) = association_index {
							event_editors.remove(index);
						}
					}
				},
				SubscriptionData::AdminUserPermissionGroupsUpdate(user_permission_group_update) => {
					match user_permission_group_update {
						AdminUserPermissionGroupData::AddUserToGroup(user_group_association) => {
							let mut user_group_associations = data_signals.user_permission_groups.modify();
							if !user_group_associations.iter().any(|association| {
								association.user.id == user_group_association.user.id
									&& association.permission_group.id == user_group_association.permission_group.id
							}) {
								user_group_associations.push(user_group_association);
							}
						}
						AdminUserPermissionGroupData::RemoveUserFromGroup(user_group_association) => {
							let mut user_group_associations = data_signals.user_permission_groups.modify();
							let association_index = user_group_associations
								.iter()
								.enumerate()
								.find(|(_, association)| {
									association.user.id == user_group_association.user.id
										&& association.permission_group.id == user_group_association.permission_group.id
								})
								.map(|(index, _)| index);
							if let Some(index) = association_index {
								user_group_associations.remove(index);
							}
						}
					}
				}
			},
			FromServerMessage::Unsubscribed(subscription_type) => {
				let mut subscription_manager = subscription_manager.lock().await;
				subscription_manager.remove_subscription(subscription_type);
			}
			FromServerMessage::SubscriptionFailure(subscription_type, failure_info) => {
				let mut subscription_manager = subscription_manager.lock().await;

				let error_message = match failure_info {
					SubscriptionFailureInfo::Error(DataError::DatabaseError) => ErrorData::new_from_string(format!(
						"Subscription to {:?} failed; a database error occurred",
						subscription_type
					)),
					SubscriptionFailureInfo::Error(DataError::ServerError) => ErrorData::new_from_string(format!(
						"Subscription to {:?} failed; a server error occurred",
						subscription_type
					)),
					SubscriptionFailureInfo::NoTarget => ErrorData::new_from_string(format!(
						"Subscription to {:?} failed; not a valid subscription target",
						subscription_type
					)),
					SubscriptionFailureInfo::NotAllowed => ErrorData::new_from_string(format!(
						"Subscription to {:?} failed; subscription not allowed",
						subscription_type
					)),
				};
				data_signals.errors.modify().push(error_message);
				subscription_manager.subscription_failure_received(subscription_type);
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
