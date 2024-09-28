// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::event_log_entry::edit::EventLogEntryEdit;
use crate::components::event_log_entry::entry::EventLogEntry as EventLogEntryView;
use crate::components::event_log_entry::typing::EventLogEntryTyping;
use crate::components::event_log_entry::UserTypingData;
use crate::page_utils::set_page_title;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use crate::websocket::WebSocketSendStream;
use chrono::Utc;
use futures::future::poll_fn;
use futures::lock::Mutex;
use futures::task::{Context, Poll, Waker};
use gloo_net::websocket::Message;
use std::collections::HashMap;
use stream_log_shared::messages::event_log::{EventLogEntry, EventLogTab, VideoEditState, VideoProcessingState};
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::subscriptions::SubscriptionType;
use stream_log_shared::messages::user::SelfUserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::{window, Event as WebEvent, ScrollIntoViewOptions, ScrollLogicalPosition};

fn add_entries_for_parent(
	entries_by_parent: &HashMap<String, Vec<EventLogEntry>>,
	entry_numbers: &mut HashMap<String, usize>,
	current_number: &mut usize,
	parent: &str,
) {
	let Some(entries) = entries_by_parent.get(parent) else {
		return;
	};
	for entry in entries.iter() {
		*current_number += 1;
		entry_numbers.insert(entry.id.clone(), *current_number);
		add_entries_for_parent(entries_by_parent, entry_numbers, current_number, &entry.id);
	}
}

#[derive(Prop)]
pub struct EventLogProps {
	id: String,
}

#[component]
async fn EventLogLoadedView<G: Html>(ctx: Scope<'_>, props: EventLogProps) -> View<G> {
	log::debug!("Starting event log load for event {}", props.id);

	let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	log::debug!("Got websocket to load event {}", props.id);

	let data: &DataSignals = use_context(ctx);

	let add_subscription_data = {
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager
			.set_subscription(SubscriptionType::EventLogData(props.id.clone()), &mut ws)
			.await
	};
	if let Err(error) = add_subscription_data {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send event subscription message.",
			error,
		));
	}
	log::debug!("Added subscription data for event {}", props.id);

	let event_subscription_data = poll_fn(|poll_context: &mut Context<'_>| {
		log::debug!(
			"Checking whether event {} is present yet in the subscription manager",
			props.id
		);
		match data.events.get().get(&props.id) {
			Some(event_subscription_data) => Poll::Ready(event_subscription_data.clone()),
			None => {
				let event_wakers: &Signal<HashMap<String, Vec<Waker>>> = use_context(ctx);
				event_wakers
					.modify()
					.entry(props.id.clone())
					.or_default()
					.push(poll_context.waker().clone());
				Poll::Pending
			}
		}
	})
	.await;

	create_effect(ctx, {
		let event_signal = event_subscription_data.event.clone();
		move || {
			let event = event_signal.get();
			let page_title = format!("{} - Log | Stream Log", event.name);
			set_page_title(&page_title);
		}
	});

	let entries_by_parent_signal = create_memo(ctx, {
		let event_log_entries = event_subscription_data.event_log_entries.clone();
		move || {
			let mut entries_by_parent: HashMap<String, Vec<EventLogEntry>> = HashMap::new();
			for event_log_entry in event_log_entries.get().iter() {
				let parent = event_log_entry.parent.clone().unwrap_or_default();
				entries_by_parent
					.entry(parent)
					.or_default()
					.push(event_log_entry.clone());
			}
			entries_by_parent
		}
	});

	let entry_numbers_signal = create_memo(ctx, || {
		let entries_by_parent = entries_by_parent_signal.get();
		let mut current_number: usize = 0;
		let mut entry_numbers: HashMap<String, usize> = HashMap::new();

		add_entries_for_parent(&entries_by_parent, &mut entry_numbers, &mut current_number, "");

		entry_numbers
	});

	let read_event_tabs_signal = create_memo(ctx, {
		let event_log_tabs = event_subscription_data.event_log_tabs.clone();
		move || (*event_log_tabs.get()).clone()
	});

	let event_signal = event_subscription_data.event.clone();
	let permission_signal = event_subscription_data.permission.clone();
	let entry_types_signal = event_subscription_data.entry_types.clone();
	let tags_signal = event_subscription_data.tags.clone();
	let log_entries = event_subscription_data.event_log_entries.clone();
	let new_log_entries = event_subscription_data.new_event_log_entries.clone();
	let available_editors = event_subscription_data.editors.clone();

	let read_event_signal = create_memo(ctx, {
		let event_signal = event_signal.clone();
		move || (*event_signal.get()).clone()
	});
	let first_tab_name_signal = create_memo(ctx, || read_event_signal.get().first_tab_name.clone());
	let read_permission_signal = create_memo(ctx, {
		let permission_signal = permission_signal.clone();
		move || *permission_signal.get()
	});
	let read_entry_types_signal = create_memo(ctx, {
		let entry_types_signal = entry_types_signal.clone();
		move || (*entry_types_signal.get()).clone()
	});
	let read_tags_signal = create_memo(ctx, {
		let tags_signal = tags_signal.clone();
		move || (*tags_signal.get()).clone()
	});
	let read_log_entries = create_memo(ctx, {
		let log_entries = log_entries.clone();
		move || (*log_entries.get()).clone()
	});
	let read_new_log_entries = create_memo(ctx, {
		let new_log_entries = new_log_entries.clone();
		move || (*new_log_entries.get()).clone()
	});
	let read_available_editors = create_memo(ctx, {
		let available_editors = available_editors.clone();
		move || (*available_editors.get()).clone()
	});

	let use_editor_view = create_memo(ctx, {
		let permission_signal = permission_signal.clone();
		let available_editors = available_editors.clone();
		move || {
			let user: &Signal<Option<SelfUserData>> = use_context(ctx);
			let user = user.get();
			let permission = permission_signal.get();
			let editors = available_editors.get();

			match (*user).as_ref() {
				Some(user) => {
					*permission == PermissionLevel::Supervisor || editors.iter().any(|editor| editor.id == user.id)
				}
				None => *permission == PermissionLevel::Supervisor,
			}
		}
	});

	let editing_log_entry: &Signal<Option<EventLogEntry>> = create_signal(ctx, None);

	let video_processing_state_filters = event_subscription_data.video_processing_state_filters.clone();
	let video_edit_state_filters = event_subscription_data.video_edit_state_filters.clone();

	let current_time = Utc::now();
	let mut current_tab: Option<&EventLogTab> = None;
	let event_log_tabs = event_subscription_data.event_log_tabs.get();
	for next_tab in event_log_tabs.iter() {
		if next_tab.start_time <= current_time {
			current_tab = Some(next_tab);
		} else {
			break;
		}
	}
	let current_tab = current_tab.cloned();
	let selected_tab = create_signal(ctx, current_tab);

	let log_entries_by_tab = create_memo(ctx, {
		let event_log_tabs = event_subscription_data.event_log_tabs.clone();
		move || {
			let entries_by_parent = entries_by_parent_signal.get();
			let tabs = event_log_tabs.get();

			let Some(entries) = entries_by_parent.get("") else {
				return HashMap::new();
			};

			let mut entries_iter = entries.iter();
			let mut tabs_iter = tabs.iter();

			let mut next_entry = entries_iter.next();
			let mut next_tab = tabs_iter.next();

			let mut entries_by_tab: HashMap<String, Vec<EventLogEntry>> = HashMap::new();
			let mut current_tab: Option<&EventLogTab> = None;

			let add_entry_to_tab = |entries_by_tab: &mut HashMap<String, Vec<EventLogEntry>>,
			                        tab_id: String,
			                        entry: EventLogEntry,
			                        entries_by_parent: &HashMap<String, Vec<EventLogEntry>>| {
				let entry_id = entry.id.clone();
				let tab_entry = entries_by_tab.entry(tab_id).or_default();
				tab_entry.push(entry);
				let mut remaining_child_entries: Vec<EventLogEntry> = entries_by_parent
					.get(&entry_id)
					.as_ref()
					.map(|entries| entries.iter().rev().cloned().collect())
					.unwrap_or_default();
				while let Some(entry) = remaining_child_entries.pop() {
					let entry_id = entry.id.clone();
					tab_entry.push(entry);
					if let Some(child_entries) = entries_by_parent.get(&entry_id) {
						for child_entry in child_entries.iter().rev() {
							remaining_child_entries.push(child_entry.clone());
						}
					}
				}
			};

			loop {
				match (next_entry, next_tab) {
					(Some(entry), Some(tab)) => {
						if entry.start_time.unwrap() < tab.start_time {
							let tab_id = current_tab.as_ref().map(|tab| tab.id.clone()).unwrap_or_default();
							add_entry_to_tab(&mut entries_by_tab, tab_id, entry.clone(), &entries_by_parent);
							next_entry = entries_iter.next();
						} else {
							current_tab = next_tab;
							next_tab = tabs_iter.next();
						}
					}
					(Some(entry), None) => {
						let tab_id = current_tab.as_ref().map(|tab| tab.id.clone()).unwrap_or_default();
						add_entry_to_tab(&mut entries_by_tab, tab_id, entry.clone(), &entries_by_parent);
						next_entry = entries_iter.next();
					}
					(None, _) => break,
				}
			}

			entries_by_tab
		}
	});

	let active_log_entries = create_memo(ctx, move || {
		let selected_tab = selected_tab.get();
		let tab_id = (*selected_tab).as_ref().map(|tab| tab.id.as_str()).unwrap_or("");
		let entries = log_entries_by_tab.get().get(tab_id).cloned().unwrap_or_default();
		let top_level_entries: Vec<EventLogEntry> =
			entries.into_iter().filter(|entry| entry.parent.is_none()).collect();
		top_level_entries
	});

	let tabs_by_entry_id = create_memo(ctx, move || {
		let entries_by_tab = log_entries_by_tab.get();
		let mut tabs_by_entry_id: HashMap<String, String> = HashMap::new();
		for (tab_id, entries) in entries_by_tab.iter() {
			for entry in entries.iter() {
				tabs_by_entry_id.insert(entry.id.clone(), tab_id.clone());
			}
		}
		tabs_by_entry_id
	});

	let can_edit = create_memo(ctx, move || permission_signal.get().can_edit());

	log::debug!("Set up loaded data signals for event {}", props.id);

	let editing_entry_parent: &Signal<Option<EventLogEntry>> = create_signal(ctx, None);
	create_effect(ctx, || {
		let editing_entry = editing_log_entry.get();
		let parent_entry = editing_entry_parent.get();
		if let Some(edit_entry) = editing_entry.as_ref() {
			if let Some(parent) = parent_entry.as_ref() {
				if edit_entry.id == parent.id {
					editing_entry_parent.set(None);
				}
			}
		}
	});

	create_effect(ctx, {
		let log_entries = log_entries.clone();
		move || {
			let log_entries = log_entries.get();
			let new_log_entries = new_log_entries.get();
			let editing_entry = editing_log_entry.get_untracked();
			let editing_entry_id = (*editing_entry).as_ref().map(|entry| entry.id.clone());
			if let Some(id) = editing_entry_id {
				if !log_entries.iter().any(|entry| entry.id == id)
					&& !new_log_entries.iter().any(|entry| entry.id == id)
				{
					editing_log_entry.set(None);
				}
			}
		}
	});

	let editing_typing_data = create_memo(ctx, {
		let typing_events = event_subscription_data.typing_events.clone();
		move || {
			let mut typing_data: HashMap<String, UserTypingData> = HashMap::new();
			let editing_entry = editing_log_entry.get();
			let editing_entry_id = (*editing_entry)
				.as_ref()
				.map(|entry| entry.id.clone())
				.unwrap_or_default();
			for typing_event in typing_events
				.get()
				.iter()
				.filter(|typing_event| typing_event.event_log_entry.id == editing_entry_id)
			{
				let (_, user_typing_data) = typing_data
					.entry(typing_event.user.id.clone())
					.or_insert((typing_event.user.clone(), HashMap::new()));
				user_typing_data.insert(typing_event.target_field, typing_event.data.clone());
			}
			typing_data
		}
	});

	let all_video_processing_states = VideoProcessingState::all_states();
	let all_video_processing_state_filters: Vec<(String, &Signal<bool>)> = all_video_processing_states
		.into_iter()
		.map(|processing_state| {
			let state_name = format!("{}", processing_state);
			let active_signal = create_signal(ctx, video_processing_state_filters.get().contains(&processing_state));

			create_effect(ctx, {
				let video_processing_state_filters = video_processing_state_filters.clone();
				move || {
					if *active_signal.get() {
						video_processing_state_filters.modify().insert(processing_state);
					} else {
						video_processing_state_filters.modify().remove(&processing_state);
					}
				}
			});

			(state_name, active_signal)
		})
		.collect();
	let all_video_processing_state_filters = create_signal(ctx, all_video_processing_state_filters);

	let all_video_edit_state_filters: Vec<(&str, &Signal<bool>)> = VideoEditState::all_states()
		.into_iter()
		.map(|edit_state| {
			let state_name = match edit_state {
				VideoEditState::NoVideo => "No Video",
				VideoEditState::MarkedForEditing => "Marked",
				VideoEditState::DoneEditing => "Done Editing",
			};
			let active_signal = create_signal(ctx, video_edit_state_filters.get().contains(&edit_state));

			create_effect(ctx, {
				let video_edit_state_filters = video_edit_state_filters.clone();
				move || {
					if *active_signal.get() {
						video_edit_state_filters.modify().insert(edit_state);
					} else {
						video_edit_state_filters.modify().remove(&edit_state);
					}
				}
			});

			(state_name, active_signal)
		})
		.collect();
	let all_video_edit_state_filters = create_signal(ctx, all_video_edit_state_filters);

	let jump_highlight_row_id = create_signal(ctx, String::new());
	let jump_id_entry = create_signal(ctx, String::new());
	let jump_handler = {
		let event_log_tabs = event_subscription_data.event_log_tabs.clone();
		move |event: WebEvent| {
			event.prevent_default();

			let jump_id = (*jump_id_entry.get()).clone();
			jump_id_entry.set(String::new());

			let tab_index = tabs_by_entry_id.get();
			let Some(tab_id) = tab_index.get(&jump_id) else {
				return;
			};
			if tab_id.is_empty() {
				selected_tab.set(None);
			} else if let Some(tab) = event_log_tabs.get().iter().find(|tab| tab.id == *tab_id) {
				selected_tab.set(Some(tab.clone()));
			}
			let jump_to_id = format!("event_log_entry_{}", jump_id);
			let Some(window) = window() else {
				return;
			};
			let Some(document) = window.document() else {
				return;
			};
			let Some(row_top_element) = document.get_element_by_id(&jump_to_id) else {
				return;
			};
			let scroll_into_view_options = ScrollIntoViewOptions::new();
			scroll_into_view_options.set_block(ScrollLogicalPosition::Center);
			row_top_element.scroll_into_view_with_scroll_into_view_options(&scroll_into_view_options);
			jump_highlight_row_id.set(jump_id);
		}
	};

	let save_message_queue: &Signal<Vec<FromClientMessage>> = create_signal(ctx, Vec::new());
	create_effect(ctx, move || {
		save_message_queue.track();
		let mut message_queue = save_message_queue.modify();
		let messages = std::mem::take(&mut *message_queue);

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			for message in messages {
				let message_json = match serde_json::to_string(&message) {
					Ok(msg) => msg,
					Err(error) => {
						let data: &DataSignals = use_context(ctx);
						data.errors.modify().push(ErrorData::new_with_error(
							"Failed to serialize event log entry update.",
							error,
						));
						continue;
					}
				};

				let send_result = ws.send(Message::Text(message_json)).await;
				if let Err(error) = send_result {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to send event log entry update.",
						error,
					));
				}
			}
		});
	});

	let visible_event_signal = event_signal.clone();
	let typing_event = event_signal.clone();
	let typing_event_log = log_entries.clone();

	let first_tab_click_handler = |_event: WebEvent| {
		selected_tab.set(None);
	};

	log::debug!("Created signals and handlers for event {}", props.id);

	let new_entries_event_subscription_data = event_subscription_data.clone();

	view! {
		ctx,
		div(id="event_log_layout") {
			div(id="event_log_header") {
				h1(id="event_log_title") { (visible_event_signal.get().name) }
				div(id="event_log_view_search") {
					form(id="event_log_jump", on:submit=jump_handler) {
						input(type="text", bind:value=jump_id_entry, placeholder="ID")
						button(type="submit") { "Jump" }
					}
				}
			}
			div(id="event_log_tabs") {
				div(
					class=if selected_tab.get().is_none() { "event_log_tab_active click" } else { "click" },
					on:click=first_tab_click_handler
				) {
					(first_tab_name_signal.get())
				}
				Keyed(
					iterable=read_event_tabs_signal,
					key=|tab| tab.id.clone(),
					view=move |ctx, tab| {
						let tab_click_handler = {
							let tab = tab.clone();
							move |_event: WebEvent| {
								selected_tab.set(Some(tab.clone()));
							}
						};

						view! {
							ctx,
							div(class={
								let selected_tab_value = selected_tab.get();
								let selected_tab_id = (*selected_tab_value).as_ref().map(|tab| tab.id.as_str()).unwrap_or("");
								if *selected_tab_id == tab.id {
									"event_log_tab_active click"
								} else {
									"click"
								}
							}, on:click=tab_click_handler) {
								(tab.name)
							}
						}
					}
				)
			}
			div(id="event_log") {
				div(id="event_log_data", class=if *use_editor_view.get() { "event_log_data_editor" } else { "" }) {
					div(class="event_log_header") { }
					div(class="event_log_header") { }
					div(class="event_log_header") { "Start" }
					div(class="event_log_header") { "End" }
					div(class="event_log_header") { "Type" }
					div(class="event_log_header") { "Description" }
					div(class="event_log_header") { "Submitter/Winner" }
					div(class="event_log_header") { "Media link" }
					div(class="event_log_header") { "Tags" }
					div(class="event_log_header") { "Poster?" }
					div(class="event_log_header") {
						div(class="event_log_column_filter_menu") {
							img(src="images/filter.png", alt="Filter video edit states", class="event_log_header_filter_icon")
							ul(class="event_log_column_filter_dropdown") {
								Keyed(
									iterable=all_video_edit_state_filters,
									key=|(state_name, _)| state_name.to_string(),
									view=|ctx, (state_name, filter_active)| {
										view! {
											ctx,
											li {
												label {
													input(type="checkbox", bind:checked=filter_active)
													span { (state_name) }
												}
											}
										}
									}
								)
							}
						}
					}
					(if *use_editor_view.get() {
						view! {
							ctx,
							div(class="event_log_header") { }
						}
					} else {
						view! { ctx, }
					})
					div(class="event_log_header") { }
					(if *use_editor_view.get() {
						view! {
							ctx,
							div(class="event_log_header") { "Editor" }
						}
					} else {
						view! { ctx, }
					})
					div(class="event_log_header") { "Notes" }
					(if *use_editor_view.get() {
						view! {
							ctx,
							div(class="event_log_header") {
								"State"
								div(class="event_log_column_filter_menu") {
									img(src="images/filter.png", alt="Filter video processing states", class="event_log_header_filter_icon")
									ul(class="event_log_column_filter_dropdown") {
										Keyed(
											iterable=all_video_processing_state_filters,
											key=|(state_name, _)| state_name.clone(),
											view=|ctx, (state_name, filter_active)| {
												view! {
													ctx,
													li {
														label {
															input(type="checkbox", bind:checked=filter_active)
															span { (state_name) }
														}
													}
												}
											}
										)
									}
								}
							}
							div(class="event_log_header") { "Video Errors" }
						}
					} else {
						view! { ctx, }
					})
					Keyed(
						iterable=active_log_entries,
						key=|entry| entry.id.clone(),
						view={
							let event_subscription_data = event_subscription_data.clone();
							move |ctx, entry| {
								let event_subscription_data=event_subscription_data.clone();

								view! {
									ctx,
									EventLogEntryView(
										entry=entry,
										jump_highlight_row_id=jump_highlight_row_id,
										event_subscription_data=event_subscription_data,
										can_edit=can_edit,
										editing_log_entry=editing_log_entry,
										read_entry_types_signal=read_entry_types_signal,
										editing_entry_parent=editing_entry_parent,
										entries_by_parent=entries_by_parent_signal,
										child_depth=0,
										entry_numbers=entry_numbers_signal,
										use_editor_view=use_editor_view
									)
								}
							}
						}
					)
					Keyed(
						iterable=read_new_log_entries,
						key=|entry| entry.id.clone(),
						view={
							let event_subscription_data = new_entries_event_subscription_data.clone();
							move |ctx, entry| {
								let event_subscription_data = event_subscription_data.clone();

								view! {
									ctx,
									EventLogEntryView(
										entry=entry,
										jump_highlight_row_id=jump_highlight_row_id,
										event_subscription_data=event_subscription_data,
										can_edit=can_edit,
										editing_log_entry=editing_log_entry,
										read_entry_types_signal=read_entry_types_signal,
										editing_entry_parent=editing_entry_parent,
										entries_by_parent=entries_by_parent_signal,
										child_depth=0,
										entry_numbers=entry_numbers_signal,
										use_editor_view=use_editor_view
									)
								}
							}
						}
					)
				}
			}
			(if *can_edit.get() {
				let typing_event = typing_event.clone();
				let typing_event_log = typing_event_log.clone();
				view! {
					ctx,
					div(id="event_log_new_entry") {
						({
							if editing_typing_data.get().is_empty() {
								view! { ctx, }
							} else {
								let typing_event = typing_event.clone();
								let typing_event_log = typing_event_log.clone();
								view! {
									ctx,
									div(id="event_log_new_entry_typing", class=if *use_editor_view.get() { "event_log_new_entry_typing_editor" } else { "" }) {
										div(class="event_log_header") {}
										div(class="event_log_header") {}
										div(class="event_log_header") { "Start" }
										div(class="event_log_header") { "End" }
										div(class="event_log_header") { "Type" }
										div(class="event_log_header") { "Description" }
										div(class="event_log_header") { "Submitter/Winner" }
										div(class="event_log_header") { "Media link" }
										div(class="event_log_header") {}
										div(class="event_log_header") {}
										div(class="event_log_header") {}
										(if *use_editor_view.get() {
											view! {
												ctx,
												div(class="event_log_header") {}
											}
										} else {
											view! { ctx, }
										})
										div(class="event_log_header") {}
										(if *use_editor_view.get() {
											view! {
												ctx,
												div(class="event_log_header") {}
											}
										} else {
											view! { ctx, }
										})
										div(class="event_log_header") { "Notes to editor" }
										(if *use_editor_view.get() {
											view! {
												ctx,
												div(class="event_log_header") {}
												div(class="event_log_header") {}
											}
										} else {
											view! { ctx, }
										})
										EventLogEntryTyping(
											event=typing_event,
											event_entry_types=read_entry_types_signal,
											event_log=typing_event_log,
											typing_data=editing_typing_data,
											use_editor_view=use_editor_view
										)
									}
								}
							}
						})
						({
							if editing_log_entry.get().is_some() {
								view! {
									ctx,
									EventLogEntryEdit(
										event=read_event_signal,
										permission_level=read_permission_signal,
										event_entry_types=read_entry_types_signal,
										event_tags=read_tags_signal,
										event_editors=read_available_editors,
										event_log_tabs=read_event_tabs_signal,
										current_tab=selected_tab,
										event_log_entries=read_log_entries,
										editing_log_entry=editing_log_entry,
										edit_parent_log_entry=editing_entry_parent,
										save_message_queue=save_message_queue
									)
								}
							} else {
								view! { ctx, }
							}
						})
					}
				}
			} else {
				view! { ctx, }
			})
		}
		datalist(id="event_entry_types") {
			Keyed(
				iterable=read_entry_types_signal,
				key=|entry_type| entry_type.id.clone(),
				view=|ctx, entry_type| {
					let type_name = entry_type.name;
					view! {
						ctx,
						option(value=type_name)
					}
				}
			)
		}
		datalist(id="event_tags") {
			Keyed(
				iterable=read_tags_signal,
				key=|tag| tag.id.clone(),
				view=|ctx, tag| {
					let tag_name = tag.name;
					view! {
						ctx,
						option(value=tag_name)
					}
				}
			)
		}
		datalist(id="editor_names") {
			Keyed(
				iterable=read_available_editors,
				key=|editor| editor.id.clone(),
				view=|ctx, editor| {
					let editor_name = editor.username;
					view! {
						ctx,
						option(value=editor_name)
					}
				}
			)
		}
	}
}

#[component]
pub fn EventLogView<G: Html>(ctx: Scope<'_>, props: EventLogProps) -> View<G> {
	// At a minimum, you need to have a user account to see this page, so we'll verify that exists
	let user_signal: &Signal<Option<SelfUserData>> = use_context(ctx);
	if user_signal.get().is_none() {
		spawn_local_scoped(ctx, async {
			navigate("/");
		});
		return view! { ctx, };
	}
	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading event log data..." }) {
			EventLogLoadedView(id=props.id)
		}
	}
}
