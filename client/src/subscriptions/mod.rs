use crate::websocket::read_websocket;
use chrono::Utc;
use futures::lock::Mutex;
use futures::stream::SplitStream;
use futures::task::Waker;
use gloo_net::websocket::futures::WebSocket;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{
	AdminApplicationData, AdminEntryTypeData, AdminEntryTypeEventData, AdminEventData, AdminEventEditorData,
	AdminEventLogTabsData, AdminInfoPageData, AdminPermissionGroupData, AdminUserPermissionGroupData, Application,
	EditorEventAssociation, EntryTypeEventAssociation, PermissionGroup, PermissionGroupEventAssociation,
	UserPermissionGroupAssociation,
};
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::{EventLogEntry, EventLogTab};
use stream_log_shared::messages::event_subscription::{EventSubscriptionData, TypingData};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::info_pages::InfoPage;
use stream_log_shared::messages::subscriptions::{
	InitialSubscriptionLoadData, SubscriptionData, SubscriptionFailureInfo, SubscriptionType,
};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::user_register::RegistrationResponse;
use stream_log_shared::messages::{DataError, FromServerMessage};
use sycamore::prelude::*;

pub mod errors;
use errors::ErrorData;

pub mod event;
use event::{EventSubscriptionSignals, EventSubscriptionSignalsInitData, TypingEvent, TypingTarget};

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

	/// List of all editor user/event pairings
	pub event_editors: RcSignal<Vec<EditorEventAssociation>>,

	/// List of all user/permission group pairings
	pub user_permission_groups: RcSignal<Vec<UserPermissionGroupAssociation>>,

	/// List of all pairings of entry types and events
	pub entry_type_event_associations: RcSignal<Vec<EntryTypeEventAssociation>>,

	/// List of all event log tabs with their associated events
	pub all_event_log_tabs: RcSignal<Vec<(Event, EventLogTab)>>,

	/// List of all applications
	pub all_applications: RcSignal<Vec<Application>>,

	/// List of all info pages
	pub all_info_pages: RcSignal<Vec<InfoPage>>,

	/// List of application auth keys to show
	pub show_application_auth_keys: RcSignal<Vec<(Application, String)>>,
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
			event_editors: create_rc_signal(Vec::new()),
			user_permission_groups: create_rc_signal(Vec::new()),
			entry_type_event_associations: create_rc_signal(Vec::new()),
			all_event_log_tabs: create_rc_signal(Vec::new()),
			all_applications: create_rc_signal(Vec::new()),
			all_info_pages: create_rc_signal(Vec::new()),
			show_application_auth_keys: create_rc_signal(Vec::new()),
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
					InitialSubscriptionLoadData::Event(event_load_data) => {
						let mut event_signals = data_signals.events.modify();
						let event_id = event_load_data.event.id.clone();
						match event_signals.entry(event_id.clone()) {
							Entry::Occupied(mut event_entry) => {
								let event_data = event_entry.get_mut();
								event_data.event.set(event_load_data.event);
								event_data.permission.set(event_load_data.permission);
								event_data.entry_types.set(event_load_data.entry_types);
								event_data.tags.set(event_load_data.tags);
								event_data.editors.set(event_load_data.editors);
								event_data.info_pages.set(event_load_data.info_pages);
								event_data.event_log_tabs.set(event_load_data.tabs);
								event_data.event_log_entries.set(event_load_data.entries);
							}
							Entry::Vacant(event_entry) => {
								let signal_data = EventSubscriptionSignalsInitData {
									event: event_load_data.event,
									permission: event_load_data.permission,
									entry_types: event_load_data.entry_types,
									tags: event_load_data.tags,
									editors: event_load_data.editors,
									info_pages: event_load_data.info_pages,
									event_log_tabs: event_load_data.tabs,
									event_log_entries: event_load_data.entries,
								};
								event_entry.insert(EventSubscriptionSignals::new(signal_data));
							}
						}
						subscription_manager
							.subscription_confirmation_received(SubscriptionType::EventLogData(event_id.clone()));

						log::debug!("Running subscription wakers for event {}", event_id);

						let event_wakers: &Signal<HashMap<String, Vec<Waker>>> = use_context(ctx);
						let event_wakers = event_wakers.modify().remove(&event_id);
						if let Some(wakers) = event_wakers {
							for waker in wakers.iter() {
								waker.wake_by_ref();
							}
						}
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
					InitialSubscriptionLoadData::AdminEventEditors(event_editors) => {
						data_signals.event_editors.set(event_editors);
						subscription_manager.subscription_confirmation_received(SubscriptionType::AdminEventEditors);
					}
					InitialSubscriptionLoadData::AdminEventLogTabs(tabs) => {
						data_signals.all_event_log_tabs.set(tabs);
						subscription_manager.subscription_confirmation_received(SubscriptionType::AdminEventLogTabs);
					}
					InitialSubscriptionLoadData::AdminApplications(applications) => {
						data_signals.all_applications.set(applications);
						subscription_manager.subscription_confirmation_received(SubscriptionType::AdminApplications);
					}
					InitialSubscriptionLoadData::AdminInfoPages(info_pages) => {
						data_signals.all_info_pages.set(info_pages);
						subscription_manager.subscription_confirmation_received(SubscriptionType::AdminInfoPages);
					}
				}
			}
			FromServerMessage::SubscriptionMessage(subscription_data) => match *subscription_data {
				SubscriptionData::EventUpdate(event, update_data) => {
					let mut events_data = data_signals.events.modify();
					let Some(event_data) = events_data.get_mut(&event.id) else {
						continue;
					};
					match *update_data {
						EventSubscriptionData::UpdateEvent => event_data.event.set(event),
						EventSubscriptionData::NewLogEntry(log_entry, creating_user) => {
							let mut event_log_entries = event_data.event_log_entries.modify();
							match event_log_entries.last() {
								Some(last_entry) => {
									if log_entry.start_time >= last_entry.start_time {
										event_log_entries.push(log_entry);
									} else {
										let insert_index = entry_insertion_index(&event_log_entries, &log_entry);
										event_log_entries.insert(insert_index, log_entry);
									}
								}
								None => event_log_entries.push(log_entry),
							};

							let mut typing_events = event_data.typing_events.modify();
							*typing_events = typing_events
								.iter()
								.filter(|typing_event| {
									typing_event.user.id != creating_user.id || typing_event.event_log_entry.is_some()
								})
								.cloned()
								.collect();
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
						EventSubscriptionData::UpdateLogEntry(log_entry, update_user) => {
							if let Some(update_user) = update_user {
								let mut typing_events = event_data.typing_events.modify();
								*typing_events = typing_events
									.iter()
									.filter(|typing_event| {
										typing_event.user.id != update_user.id
											|| typing_event.event_log_entry.as_ref().map(|entry| &entry.id)
												!= Some(&log_entry.id)
									})
									.cloned()
									.collect();
							}

							let mut log_entries = event_data.event_log_entries.modify();
							let existing_entry_index = log_entries
								.iter_mut()
								.enumerate()
								.find(|(_, entry)| entry.id == log_entry.id)
								.map(|(index, _)| index);
							if let Some(index) = existing_entry_index {
								if log_entries[index].start_time != log_entry.start_time
									|| log_entries[index].manual_sort_key != log_entry.manual_sort_key
									|| log_entries[index].created_at != log_entry.created_at
								{
									log_entries.remove(index);
									let new_index = entry_insertion_index(&log_entries, &log_entry);
									log_entries.insert(new_index, log_entry);
								} else {
									log_entries[index] = log_entry;
								}
							}
						}
						EventSubscriptionData::Typing(typing_data) => {
							let user: &Signal<Option<UserData>> = use_context(ctx);
							// If we're not logged in, we shouldn't be receiving typing data.
							let user = user.get();
							let Some(user) = user.as_ref() else {
								continue;
							};
							// If we're not subscribed to the event in question, we don't need to track this data.
							let data_events = data_signals.events.get();
							let Some(event_data) = data_events.get(&event.id) else {
								continue;
							};
							match typing_data {
								TypingData::Parent(event_log_entry, parent_entry_id, typing_user) => {
									if user.id != typing_user.id {
										handle_typing_data(
											event_data,
											event_log_entry,
											parent_entry_id,
											typing_user,
											TypingTarget::Parent,
										);
									}
								}
								TypingData::StartTime(event_log_entry, typed_time, typing_user) => {
									if user.id != typing_user.id {
										handle_typing_data(
											event_data,
											event_log_entry,
											typed_time,
											typing_user,
											TypingTarget::StartTime,
										);
									}
								}
								TypingData::EndTime(event_log_entry, typed_time, typing_user) => {
									if user.id != typing_user.id {
										handle_typing_data(
											event_data,
											event_log_entry,
											typed_time,
											typing_user,
											TypingTarget::EndTime,
										);
									}
								}
								TypingData::EntryType(event_log_entry, typed_type, typing_user) => {
									if user.id != typing_user.id {
										handle_typing_data(
											event_data,
											event_log_entry,
											typed_type,
											typing_user,
											TypingTarget::EntryType,
										);
									}
								}
								TypingData::Description(event_log_entry, typed_description, typing_user) => {
									if user.id != typing_user.id {
										handle_typing_data(
											event_data,
											event_log_entry,
											typed_description,
											typing_user,
											TypingTarget::Description,
										);
									}
								}
								TypingData::MediaLinks(event_log_entry, typed_link, typing_user) => {
									if user.id != typing_user.id {
										handle_typing_data(
											event_data,
											event_log_entry,
											typed_link,
											typing_user,
											TypingTarget::MediaLink,
										);
									}
								}
								TypingData::SubmitterWinner(event_log_entry, typed_name, typing_user) => {
									if user.id != typing_user.id {
										handle_typing_data(
											event_data,
											event_log_entry,
											typed_name,
											typing_user,
											TypingTarget::SubmitterWinner,
										);
									}
								}
								TypingData::NotesToEditor(event_log_entry, typed_notes, typing_user) => {
									if user.id != typing_user.id {
										handle_typing_data(
											event_data,
											event_log_entry,
											typed_notes,
											typing_user,
											TypingTarget::NotesToEditor,
										);
									}
								}
								TypingData::Clear(event_log_entry, typing_user) => {
									event_data.typing_events.modify().retain(|typing_event| {
										typing_event.user != typing_user
											|| typing_event.event_log_entry != event_log_entry
									})
								}
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
						EventSubscriptionData::UpdateInfoPage(info_page) => {
							let mut info_pages = event_data.info_pages.modify();
							let info_page_entry = info_pages.iter_mut().find(|page| page.id == info_page.id);
							match info_page_entry {
								Some(entry) => *entry = info_page,
								None => info_pages.push(info_page),
							}
						}
						EventSubscriptionData::DeleteInfoPage(info_page) => {
							let mut info_pages = event_data.info_pages.modify();
							let info_page_index = info_pages
								.iter()
								.enumerate()
								.find(|(_, page)| page.id == info_page.id)
								.map(|(index, _)| index);
							if let Some(index) = info_page_index {
								info_pages.remove(index);
							}
						}
						EventSubscriptionData::UpdateTab(tab) => {
							let mut tabs = event_data.event_log_tabs.modify();
							let tab_entry = tabs.iter_mut().find(|t| tab.id == t.id);
							match tab_entry {
								Some(entry) => *entry = tab,
								None => {
									match tabs.binary_search_by_key(&tab.start_time, |section| section.start_time) {
										Ok(index) => tabs.insert(index, tab),
										Err(index) => tabs.insert(index, tab),
									}
								}
							}
						}
						EventSubscriptionData::DeleteTab(tab) => event_data
							.event_log_tabs
							.modify()
							.retain(|tab_entry| tab_entry.id != tab.id),
						EventSubscriptionData::UpdateTag(tag) => {
							let mut tags = event_data.tags.modify();
							let tag_entry = tags.iter_mut().find(|t| t.id == tag.id);
							match tag_entry {
								Some(entry) => *entry = tag,
								None => tags.push(tag),
							}
						}
						EventSubscriptionData::RemoveTag(tag) => {
							let mut tags = event_data.tags.modify();
							let tag_index = tags
								.iter()
								.enumerate()
								.find(|(_, t)| t.id == tag.id)
								.map(|(index, _)| index);
							if let Some(index) = tag_index {
								tags.remove(index);
							}
						}
					}
				}
				SubscriptionData::UserUpdate(user_update) => {
					let user_signal: &Signal<Option<UserData>> = use_context(ctx);
					user_signal.set(Some(user_update.user));
					let mut available_events = user_update.available_events;
					available_events.sort_unstable_by(|a, b| a.start_time.cmp(&b.start_time).reverse());
					data_signals.available_events.set(available_events);
				}
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
				SubscriptionData::AdminEventLogTabsUpdate(event_log_tabs_update) => match event_log_tabs_update {
					AdminEventLogTabsData::AddTab(event, new_tab) => {
						data_signals.all_event_log_tabs.modify().push((event, new_tab))
					}
					AdminEventLogTabsData::UpdateTab(new_tab_data) => {
						let mut event_log_tabs = data_signals.all_event_log_tabs.modify();
						let tab_entry = event_log_tabs.iter_mut().find(|entry| entry.1.id == new_tab_data.id);
						if let Some(entry) = tab_entry {
							entry.1 = new_tab_data;
						}
					}
					AdminEventLogTabsData::DeleteTab(tab) => data_signals
						.all_event_log_tabs
						.modify()
						.retain(|entry| entry.1.id != tab.id),
				},
				SubscriptionData::AdminApplicationsUpdate(application_update) => match application_update {
					AdminApplicationData::UpdateApplication(application) => {
						let mut all_applications = data_signals.all_applications.modify();
						let application_entry = all_applications.iter_mut().find(|app| app.id == application.id);
						match application_entry {
							Some(app) => *app = application,
							None => all_applications.push(application),
						}
					}
					AdminApplicationData::ShowApplicationAuthKey(application, auth_key) => {
						{
							let mut application_auth_keys = data_signals.show_application_auth_keys.modify();
							let auth_key_entry = application_auth_keys
								.iter_mut()
								.find(|(app, _)| app.id == application.id);
							match auth_key_entry {
								Some(entry) => *entry = (application, auth_key),
								None => application_auth_keys.push((application, auth_key)),
							}
						}
						data_signals.show_application_auth_keys.trigger_subscribers();
					}
					AdminApplicationData::RevokeApplication(application) => {
						let mut all_applications = data_signals.all_applications.modify();
						let application_index = all_applications
							.iter()
							.enumerate()
							.find(|(_, app)| app.id == application.id)
							.map(|(index, _)| index);
						if let Some(index) = application_index {
							all_applications.remove(index);
						}

						let mut application_auth_keys = data_signals.show_application_auth_keys.modify();
						let auth_key_index = application_auth_keys
							.iter()
							.enumerate()
							.find(|(_, (app, _))| app.id == application.id)
							.map(|(index, _)| index);
						if let Some(index) = auth_key_index {
							application_auth_keys.remove(index);
						}
					}
				},
				SubscriptionData::AdminInfoPagesUpdate(info_pages_update) => match info_pages_update {
					AdminInfoPageData::UpdateInfoPage(info_page) => {
						let mut all_info_pages = data_signals.all_info_pages.modify();
						let info_page_entry = all_info_pages.iter_mut().find(|page| page.id == info_page.id);
						match info_page_entry {
							Some(entry) => *entry = info_page,
							None => all_info_pages.push(info_page),
						}
					}
					AdminInfoPageData::DeleteInfoPage(info_page) => {
						let mut all_info_pages = data_signals.all_info_pages.modify();
						let info_page_index = all_info_pages
							.iter()
							.enumerate()
							.find(|(_, page)| page.id == info_page.id)
							.map(|(index, _)| index);
						if let Some(index) = info_page_index {
							all_info_pages.remove(index);
						}
					}
				},
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

fn handle_typing_data(
	event_data: &EventSubscriptionSignals,
	event_log_entry: Option<EventLogEntry>,
	typed_data: String,
	typing_user: UserData,
	target_field: TypingTarget,
) {
	let mut typing_events = event_data.typing_events.modify();
	if typed_data.is_empty() && target_field != TypingTarget::Parent {
		let event_index = typing_events
			.iter()
			.enumerate()
			.find(|(_, typing_event)| {
				typing_event.event_log_entry.as_ref().map(|entry| &entry.id)
					== event_log_entry.as_ref().map(|entry| &entry.id)
					&& typing_event.user.id == typing_user.id
					&& typing_event.target_field == target_field
			})
			.map(|(index, _)| index);
		if let Some(index) = event_index {
			typing_events.remove(index);
		}
	} else {
		let mut found_exact_event = false;
		for typing_event in typing_events.iter_mut() {
			if typing_event.event_log_entry.as_ref().map(|entry| &entry.id)
				== event_log_entry.as_ref().map(|entry| &entry.id)
				&& typing_event.user.id == typing_user.id
			{
				typing_event.time_received = Utc::now();

				if typing_event.target_field == target_field {
					typing_event.data = typed_data.clone();
					found_exact_event = true;
				}
			}
		}
		if !found_exact_event {
			typing_events.push(TypingEvent {
				event_log_entry,
				user: typing_user,
				target_field,
				data: typed_data,
				time_received: Utc::now(),
			});
		}
	}
}

fn entry_insertion_index(entries: &[EventLogEntry], log_entry_to_insert: &EventLogEntry) -> usize {
	match entries.binary_search_by(|check_entry| {
		check_entry
			.start_time
			.cmp(&log_entry_to_insert.start_time)
			.then_with(
				|| match (check_entry.manual_sort_key, log_entry_to_insert.manual_sort_key) {
					(Some(check_sort_key), Some(log_entry_sort_key)) => check_sort_key.cmp(&log_entry_sort_key),
					(Some(_), None) => Ordering::Less,
					(None, Some(_)) => Ordering::Greater,
					(None, None) => Ordering::Equal,
				},
			)
			.then_with(|| check_entry.created_at.cmp(&log_entry_to_insert.created_at))
	}) {
		Ok(mut found_entry_index) => {
			while found_entry_index < entries.len()
				&& entries[found_entry_index].start_time == log_entry_to_insert.start_time
				&& entries[found_entry_index].manual_sort_key == log_entry_to_insert.manual_sort_key
				&& entries[found_entry_index].created_at == log_entry_to_insert.created_at
			{
				found_entry_index += 1;
			}
			found_entry_index
		}
		Err(new_insert_index) => new_insert_index,
	}
}
