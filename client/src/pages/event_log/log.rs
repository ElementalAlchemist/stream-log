use crate::components::event_log_entry::edit::EventLogEntryEdit;
use crate::components::event_log_entry::entry::EventLogEntry as EventLogEntryView;
use crate::components::event_log_entry::typing::EventLogEntryTyping;
use crate::components::event_log_entry::UserTypingData;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use chrono::{DateTime, Utc};
use futures::future::poll_fn;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::task::{Context, Poll, Waker};
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::event_log::{EventLogEntry, EventLogSection, VideoEditState, VideoState};
use stream_log_shared::messages::event_subscription::EventSubscriptionUpdate;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use web_sys::{window, Event as WebEvent};

#[derive(Clone, Debug, Eq, PartialEq)]
enum LogLineData {
	Section(EventLogSection),
	Entry(Box<EventLogEntry>),
}

impl LogLineData {
	fn id(&self) -> String {
		match self {
			Self::Section(section) => section.id.clone(),
			Self::Entry(entry) => entry.id.clone(),
		}
	}
}

#[derive(Prop)]
pub struct EventLogProps {
	id: String,
}

#[component]
async fn EventLogLoadedView<G: Html>(ctx: Scope<'_>, props: EventLogProps) -> View<G> {
	log::debug!("Starting event log load for event {}", props.id);

	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
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
				let event_wakers: &Signal<HashMap<String, Waker>> = use_context(ctx);
				event_wakers
					.modify()
					.insert(props.id.clone(), poll_context.waker().clone());
				Poll::Pending
			}
		}
	})
	.await;

	let entries_by_parent_signal = create_memo(ctx, {
		let event_log_entries = event_subscription_data.event_log_entries.clone();
		move || {
			let mut entries_by_parent: HashMap<String, Vec<EventLogEntry>> = HashMap::new();
			for event_log_entry in event_log_entries.get().iter() {
				let Some(parent) = event_log_entry.parent.clone() else {
					continue;
				};
				entries_by_parent
					.entry(parent)
					.or_default()
					.push(event_log_entry.clone());
			}
			entries_by_parent
		}
	});

	let event_signal = event_subscription_data.event.clone();
	let permission_signal = event_subscription_data.permission.clone();
	let entry_types_signal = event_subscription_data.entry_types.clone();
	let tags_signal = event_subscription_data.tags.clone();
	let log_entries = event_subscription_data.event_log_entries.clone();
	let available_editors = event_subscription_data.editors;

	let read_event_signal = create_memo(ctx, {
		let event_signal = event_signal.clone();
		move || (*event_signal.get()).clone()
	});
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
	let read_available_editors = create_memo(ctx, {
		let available_editors = available_editors.clone();
		move || (*available_editors.get()).clone()
	});

	let active_state_filters: &Signal<HashSet<Option<VideoState>>> = create_signal(ctx, HashSet::new());

	let log_lines = create_memo(ctx, move || {
		let entries: Vec<EventLogEntry> = event_subscription_data
			.event_log_entries
			.get()
			.iter()
			.filter(|entry| entry.parent.is_none())
			.cloned()
			.collect();
		let sections = event_subscription_data.event_log_sections.get();
		let state_filters = active_state_filters.get();

		let mut entries_iter = entries.iter();
		let mut sections_iter = sections.iter();

		let mut next_entry = entries_iter.next();
		let mut next_section = sections_iter.next();

		let mut log_lines: Vec<LogLineData> = Vec::new();

		loop {
			match (next_entry, next_section) {
				(Some(entry), Some(section)) => {
					if entry.start_time < section.start_time {
						if state_filters.is_empty() || state_filters.contains(&entry.video_state) {
							log_lines.push(LogLineData::Entry(Box::new(entry.clone())));
						}
						next_entry = entries_iter.next();
					} else {
						if let Some(LogLineData::Section(existing_section)) = log_lines.last_mut() {
							*existing_section = section.clone();
						} else {
							log_lines.push(LogLineData::Section(section.clone()));
						}
						next_section = sections_iter.next();
					}
				}
				(Some(entry), None) => {
					if state_filters.is_empty() || state_filters.contains(&entry.video_state) {
						log_lines.push(LogLineData::Entry(Box::new(entry.clone())));
					}
					next_entry = entries_iter.next();
				}
				(None, _) => break,
			}
		}

		while let Some(LogLineData::Section(_)) = log_lines.last() {
			log_lines.pop();
		}

		log_lines
	});

	let tags_by_name_index = create_memo(ctx, move || {
		let name_index: HashMap<String, Tag> = tags_signal
			.get()
			.iter()
			.map(|tag| (tag.name.clone(), tag.clone()))
			.collect();
		name_index
	});
	let editors_by_name_index = create_memo(ctx, move || {
		let name_index: HashMap<String, UserData> = available_editors
			.get()
			.iter()
			.map(|editor| (editor.username.clone(), editor.clone()))
			.collect();
		name_index
	});
	let can_edit = create_memo(ctx, move || permission_signal.get().can_edit());

	log::debug!("Set up loaded data signals for event {}", props.id);

	let new_event_log_entry: &Signal<Option<EventLogEntry>> = create_signal(ctx, None);
	let new_entry_start_time = create_signal(ctx, Utc::now());
	let new_entry_end_time: &Signal<Option<DateTime<Utc>>> = create_signal(ctx, None);
	let new_entry_type = create_signal(ctx, String::new());
	let new_entry_description = create_signal(ctx, String::new());
	let new_entry_media_link = create_signal(ctx, String::new());
	let new_entry_submitter_or_winner = create_signal(ctx, String::new());
	let new_entry_tags: &Signal<Vec<Tag>> = create_signal(ctx, Vec::new());
	let new_entry_video_edit_state = create_signal(ctx, VideoEditState::NoVideo);
	let new_entry_poster_moment = create_signal(ctx, false);
	let new_entry_notes_to_editor = create_signal(ctx, String::new());
	let new_entry_editor: &Signal<Option<UserData>> = create_signal(ctx, None);
	let new_entry_marked_incomplete = create_signal(ctx, false);
	let new_entry_parent: &Signal<Option<EventLogEntry>> = create_signal(ctx, None);
	let new_entry_sort_key: &Signal<Option<i32>> = create_signal(ctx, None);
	let new_entry_typing_data = create_memo(ctx, {
		let typing_events = event_subscription_data.typing_events.clone();
		move || {
			let mut typing_data: HashMap<String, UserTypingData> = HashMap::new();
			for typing_event in typing_events
				.get()
				.iter()
				.filter(|typing_event| typing_event.event_log_entry.is_none())
			{
				let (_, user_typing_data) = typing_data
					.entry(typing_event.user.id.clone())
					.or_insert((typing_event.user.clone(), HashMap::new()));
				user_typing_data.insert(typing_event.target_field, typing_event.data.clone());
			}
			typing_data
		}
	});

	let new_entry_close_handler = {
		let event_signal = event_signal.clone();
		move |count: u8| {
			let event_signal = event_signal.clone();

			let start_time = *new_entry_start_time.get();
			let end_time = *new_entry_end_time.get();
			let entry_type = (*new_entry_type.get()).clone();
			let description = (*new_entry_description.get()).clone();
			let media_link = (*new_entry_media_link.get()).clone();
			let submitter_or_winner = (*new_entry_submitter_or_winner.get()).clone();
			let tags = (*new_entry_tags.get()).clone();
			let video_edit_state = *new_entry_video_edit_state.get();
			let poster_moment = *new_entry_poster_moment.get();
			let notes_to_editor = (*new_entry_notes_to_editor.get()).clone();
			let editor = (*new_entry_editor.get()).clone();
			let marked_incomplete = *new_entry_marked_incomplete.get();
			let parent = (*new_entry_parent.get()).as_ref().map(|entry| entry.id.clone());
			let manual_sort_key = *new_entry_sort_key.get();
			let new_event_log_entry = EventLogEntry {
				id: String::new(),
				start_time,
				end_time,
				entry_type,
				description,
				media_link,
				submitter_or_winner,
				tags,
				notes_to_editor,
				editor_link: None,
				editor,
				video_link: None,
				marked_incomplete,
				parent,
				created_at: Utc::now(),
				manual_sort_key,
				video_state: None,
				video_errors: String::new(),
				poster_moment,
				video_edit_state,
			};

			spawn_local_scoped(ctx, async move {
				let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
				let mut ws = ws_context.lock().await;

				let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
					(*event_signal.get()).clone(),
					Box::new(EventSubscriptionUpdate::NewLogEntry(new_event_log_entry, count)),
				)));
				let message_json = match serde_json::to_string(&message) {
					Ok(msg) => msg,
					Err(error) => {
						data.errors.modify().push(ErrorData::new_with_error(
							"Failed to serialize new log entry submission.",
							error,
						));
						return;
					}
				};
				if let Err(error) = ws.send(Message::Text(message_json)).await {
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to send new log entry submission.",
						error,
					));
				}
			});
		}
	};

	let expanded_sections: &Signal<HashMap<String, RcSignal<bool>>> = create_signal(ctx, HashMap::new());

	let sections_by_row_id = create_memo(ctx, || {
		let mut row_sections: HashMap<String, String> = HashMap::new();
		let mut current_section_id = String::new();
		for line_data in log_lines.get().iter() {
			match line_data {
				LogLineData::Section(section) => current_section_id = section.id.clone(),
				LogLineData::Entry(entry) => {
					row_sections.insert(entry.id.clone(), current_section_id.clone());
				}
			}
		}
		row_sections
	});

	let expand_all_handler = |_event: WebEvent| {
		for line_data in log_lines.get().iter() {
			if let LogLineData::Section(section) = line_data {
				match expanded_sections.modify().entry(section.id.clone()) {
					Entry::Occupied(entry) => entry.get().set(true),
					Entry::Vacant(entry) => {
						entry.insert(create_rc_signal(true));
					}
				}
			}
		}
	};
	let collapse_all_handler = |_event: WebEvent| {
		for line_data in log_lines.get().iter() {
			if let LogLineData::Section(section) = line_data {
				match expanded_sections.modify().entry(section.id.clone()) {
					Entry::Occupied(entry) => entry.get().set(false),
					Entry::Vacant(entry) => {
						entry.insert(create_rc_signal(false));
					}
				}
			}
		}
	};

	let mut all_video_states = vec![None];
	for video_state in VideoState::all_states() {
		all_video_states.push(Some(video_state));
	}
	let all_video_state_filters: Vec<(String, &Signal<bool>)> = all_video_states
		.iter()
		.map(|state| {
			let state_name = match state {
				Some(state) => format!("{}", state),
				None => String::from("(empty)"),
			};
			let active_signal = create_signal(ctx, active_state_filters.get().contains(state));

			let state = *state;
			create_effect(ctx, move || {
				if *active_signal.get() {
					active_state_filters.modify().insert(state);
				} else {
					active_state_filters.modify().remove(&state);
				}
			});

			(state_name, active_signal)
		})
		.collect();
	let all_video_state_filters = create_signal(ctx, all_video_state_filters);

	let jump_highlight_row_id = create_signal(ctx, String::new());
	let jump_id_entry = create_signal(ctx, String::new());
	let jump_handler = |event: WebEvent| {
		event.prevent_default();

		let jump_id = (*jump_id_entry.get()).clone();
		jump_id_entry.set(String::new());

		let section_index = sections_by_row_id.get();
		let Some(section_id) = section_index.get(&jump_id) else {
			return;
		};
		if !section_id.is_empty() {
			if let Some(section_expand_signal) = expanded_sections.get().get(section_id) {
				section_expand_signal.set(true);
			}
		}
		let jump_to_id = format!("event_log_entry_{}", jump_id);
		let Some(window) = window() else {
			return;
		};
		let Some(document) = window.document() else {
			return;
		};
		let Some(row_element) = document.get_element_by_id(&jump_to_id) else {
			return;
		};
		// The row doesn't have a size, so the browser won't scroll to it. As such, we pick a child element to which we
		// can scroll.
		let Some(cell_element) = row_element.first_element_child() else {
			return;
		};
		cell_element.scroll_into_view();
		jump_highlight_row_id.set(jump_id);
	};

	let visible_event_signal = event_signal.clone();
	let typing_event = event_signal.clone();
	let typing_event_log = log_entries.clone();

	log::debug!("Created signals and handlers for event {}", props.id);

	view! {
		ctx,
		div(id="event_log_layout") {
			div(id="event_log_header") {
				h1(id="event_log_title") { (visible_event_signal.get().name) }
				div(id="event_log_view_settings") {
					div(id="event_log_view_settings_section_control") {
						a(id="event_log_expand_all", class="click", on:click=expand_all_handler) {
							"Expand All"
						}
						a(id="event_log_collapse_all", class="click", on:click=collapse_all_handler) {
							"Collapse All"
						}
					}
					div(id="event_log_view_settings_filter") {
						div(id="event_log_view_settings_filter_video_state") {
							"Video State Filter"
							ul(class="event_log_view_settings_filter_dropdown") {
								Keyed(
									iterable=all_video_state_filters,
									key=|(state_name, _)| state_name.clone(),
									view=|ctx, (state_name, filter_active)| {
										view! {
											ctx,
											li {
												label {
													input(type="checkbox", bind:checked=filter_active)
													(state_name)
												}
											}
										}
									}
								)
							}
						}
					}
					form(id="event_log_jump", on:submit=jump_handler) {
						input(type="text", bind:value=jump_id_entry, placeholder="ID")
						button(type="submit") { "Jump" }
					}
				}
			}
			div(id="event_log") {
				div(id="event_log_data") {
					div(class="event_log_header") { }
					div(class="event_log_header") { "Start" }
					div(class="event_log_header") { "End" }
					div(class="event_log_header") { "Type" }
					div(class="event_log_header") { "Description" }
					div(class="event_log_header") { "Submitter/Winner" }
					div(class="event_log_header") { "Media link" }
					div(class="event_log_header") { "Tags" }
					div(class="event_log_header") { "Poster?" }
					div(class="event_log_header") { }
					div(class="event_log_header") { }
					div(class="event_log_header") { }
					div(class="event_log_header") { "Editor" }
					div(class="event_log_header") { "Notes to editor" }
					div(class="event_log_header") { "State" }
					div(class="event_log_header") { "Video Errors" }
					Keyed(
						iterable=log_lines,
						key=|line| line.id(),
						view={
							let event_signal = event_signal.clone();
							let entry_types_signal = entry_types_signal.clone();
							let log_entries = log_entries.clone();
							let typing_events = event_subscription_data.typing_events.clone();
							move |ctx, line| {
								let event_signal = event_signal.clone();
								let entry_types_signal = entry_types_signal.clone();
								let log_entries = log_entries.clone();
								let typing_events = typing_events.clone();

								match line {
									LogLineData::Section(section) => {
										let expanded_signal = match expanded_sections.get().get(&section.id) {
											Some(expanded_signal) => expanded_signal.clone(),
											None => {
												let signal = create_rc_signal(true);
												expanded_sections.modify().insert(section.id.clone(), signal.clone());
												signal
											}
										};
										let expand_click_handler = {
											let expanded_signal = expanded_signal.clone();
											move |_event: WebEvent| {
												expanded_signal.set(!*expanded_signal.get());
											}
										};
										let section_name = section.name.clone();
										view! {
											ctx,
											div(class="event_log_section_header") {
												div(class="event_log_section_header_name") {
													h2 {
														(section_name)
													}
												}
												div(class="event_log_section_collapse") {
													a(class="click", on:click=expand_click_handler) {
														(if *expanded_signal.get() {
															"[-]"
														} else {
															"[+]"
														})
													}
												}
											}
										}
									}
									LogLineData::Entry(entry) => {
										let section_id = sections_by_row_id.get().get(&entry.id).cloned().unwrap_or_default();
										let expanded_signal = if section_id.is_empty() {
											create_rc_signal(true)
										} else {
											match expanded_sections.get().get(&section_id) {
												Some(expanded_signal) => expanded_signal.clone(),
												None => {
													let signal = create_rc_signal(true);
													expanded_sections.modify().insert(section_id.clone(), signal.clone());
													signal
												}
											}
										};
										view! {
											ctx,
											(if *expanded_signal.get() {
												let entry = (*entry).clone();
												let event_signal = event_signal.clone();
												let entry_types_signal = entry_types_signal.clone();
												let log_entries = log_entries.clone();
												let typing_events = typing_events.clone();
												view! {
													ctx,
													EventLogEntryView(
														entry=entry,
														jump_highlight_row_id=jump_highlight_row_id,
														event_signal=event_signal,
														permission_level=read_permission_signal,
														entry_types_signal=entry_types_signal,
														all_log_entries=log_entries,
														event_typing_events_signal=typing_events,
														can_edit=can_edit,
														tags_by_name_index=tags_by_name_index,
														editors_by_name_index=editors_by_name_index,
														read_event_signal=read_event_signal,
														read_entry_types_signal=read_entry_types_signal,
														new_entry_parent=new_entry_parent,
														entries_by_parent=entries_by_parent_signal,
														child_depth=0
													)
												}
											} else {
												view! { ctx, }
											})
										}
									}
								}
							}
						}
					)
				}
			}
			(if *can_edit.get() {
				let new_entry_close_handler = new_entry_close_handler.clone();
				let typing_event = typing_event.clone();
				let typing_event_log = typing_event_log.clone();
				view! {
					ctx,
					div(id="event_log_new_entry") {
						({
							if new_entry_typing_data.get().is_empty() {
								view! { ctx, }
							} else {
								let typing_event = typing_event.clone();
								let typing_event_log = typing_event_log.clone();
								view! {
									ctx,
									div(id="event_log_new_entry_typing") {
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
										div(class="event_log_header") {}
										div(class="event_log_header") {}
										div(class="event_log_header") {}
										div(class="event_log_header") { "Notes to editor" }
										div(class="event_log_header") {}
										div(class="event_log_header") {}
										EventLogEntryTyping(event=typing_event, event_entry_types=read_entry_types_signal, event_log=typing_event_log, typing_data=new_entry_typing_data)
									}
								}
							}
						})
						EventLogEntryEdit(
							event=read_event_signal,
							permission_level=read_permission_signal,
							event_entry_types=read_entry_types_signal,
							event_tags_name_index=tags_by_name_index,
							entry_types_datalist_id="event_entry_types",
							event_log_entry=new_event_log_entry,
							tags_datalist_id="event_tags",
							start_time=new_entry_start_time,
							end_time=new_entry_end_time,
							entry_type=new_entry_type,
							description=new_entry_description,
							media_link=new_entry_media_link,
							submitter_or_winner=new_entry_submitter_or_winner,
							tags=new_entry_tags,
							video_edit_state=new_entry_video_edit_state,
							poster_moment=new_entry_poster_moment,
							notes_to_editor=new_entry_notes_to_editor,
							editor=new_entry_editor,
							editor_name_index=editors_by_name_index,
							editor_name_datalist_id="editor_names",
							marked_incomplete=new_entry_marked_incomplete,
							parent_log_entry=new_entry_parent,
							sort_key=new_entry_sort_key,
							close_handler=new_entry_close_handler
						)
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
	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading event log data..." }) {
			EventLogLoadedView(id=props.id)
		}
	}
}
