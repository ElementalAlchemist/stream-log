use crate::components::event_log_entry::{
	EventLogEntry as EventLogEntryView, EventLogEntryEdit, EventLogEntryTyping, UserTypingData,
};
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
use std::collections::HashMap;
use stream_log_shared::messages::event_log::{EventLogEntry, EventLogSection};
use stream_log_shared::messages::event_subscription::EventSubscriptionUpdate;
use stream_log_shared::messages::permissions::PermissionLevel;
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
			.set_subscriptions(
				vec![
					SubscriptionType::EventLogData(props.id.clone()),
					SubscriptionType::AvailableTags,
				],
				&mut ws,
			)
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
				let Some(parent) = event_log_entry.parent.clone() else { continue; };
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
	let tags_signal = data.available_tags.clone();
	let log_entries = event_subscription_data.event_log_entries.clone();
	let available_editors = event_subscription_data.editors;

	let read_event_signal = create_memo(ctx, {
		let event_signal = event_signal.clone();
		move || (*event_signal.get()).clone()
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

	let log_lines = create_memo(ctx, move || {
		let entries: Vec<EventLogEntry> = event_subscription_data
			.event_log_entries
			.get()
			.iter()
			.filter(|entry| entry.parent.is_none())
			.cloned()
			.collect();
		let sections = event_subscription_data.event_log_sections.get();

		let mut entries_iter = entries.iter();
		let mut sections_iter = sections.iter();

		let mut next_entry = entries_iter.next();
		let mut next_section = sections_iter.next();

		let mut log_lines: Vec<LogLineData> = Vec::new();

		loop {
			match (next_entry, next_section) {
				(Some(entry), Some(section)) => {
					if entry.start_time < section.start_time {
						log_lines.push(LogLineData::Entry(Box::new(entry.clone())));
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
					log_lines.push(LogLineData::Entry(Box::new(entry.clone())));
					next_entry = entries_iter.next();
				}
				(None, _) => break,
			}
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
	let can_edit = create_memo(ctx, move || *permission_signal.get() == PermissionLevel::Edit);

	log::debug!("Set up loaded data signals for event {}", props.id);

	let new_event_log_entry: &Signal<Option<EventLogEntry>> = create_signal(ctx, None);
	let new_entry_start_time = create_signal(ctx, Utc::now());
	let new_entry_end_time: &Signal<Option<DateTime<Utc>>> = create_signal(ctx, None);
	let new_entry_type = create_signal(ctx, String::new());
	let new_entry_description = create_signal(ctx, String::new());
	let new_entry_media_link = create_signal(ctx, String::new());
	let new_entry_submitter_or_winner = create_signal(ctx, String::new());
	let new_entry_tags: &Signal<Vec<Tag>> = create_signal(ctx, Vec::new());
	let new_entry_make_video = create_signal(ctx, false);
	let new_entry_notes_to_editor = create_signal(ctx, String::new());
	let new_entry_editor: &Signal<Option<UserData>> = create_signal(ctx, None);
	let new_entry_highlighted = create_signal(ctx, false);
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
			let submitter_or_winner = (*new_entry_media_link.get()).clone();
			let tags = (*new_entry_tags.get()).clone();
			let make_video = *new_entry_make_video.get();
			let notes_to_editor = (*new_entry_notes_to_editor.get()).clone();
			let editor = (*new_entry_editor.get()).clone();
			let highlighted = *new_entry_highlighted.get();
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
				make_video,
				notes_to_editor,
				editor_link: None,
				editor,
				video_link: None,
				highlighted,
				parent,
				created_at: Utc::now(),
				manual_sort_key,
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

	let jump_highlight_row_id = create_signal(ctx, String::new());
	let jump_id_entry = create_signal(ctx, String::new());
	let jump_handler = |event: WebEvent| {
		event.prevent_default();

		let jump_id = (*jump_id_entry.get()).clone();
		jump_id_entry.set(String::new());

		let section_index = sections_by_row_id.get();
		let Some(section_id) = section_index.get(&jump_id) else { return; };
		if !section_id.is_empty() {
			if let Some(section_expand_signal) = expanded_sections.get().get(section_id) {
				section_expand_signal.set(true);
			}
		}
		let jump_to_id = format!("event_log_entry_{}", jump_id);
		let Some(window) = window() else { return; };
		let Some(document) = window.document() else { return; };
		let Some(row_element) = document.get_element_by_id(&jump_to_id) else { return; };
		// The row doesn't have a size, so the browser won't scroll to it. As such, we pick a child element to which we
		// can scroll.
		let Some(cell_element) = row_element.first_element_child() else { return; };
		cell_element.scroll_into_view();
		jump_highlight_row_id.set(jump_id);
	};

	let visible_event_signal = event_signal.clone();

	log::debug!("Created signals and handlers for event {}", props.id);

	view! {
		ctx,
		div(id="event_log_layout") {
			div(id="event_log_header") {
				h1(id="event_log_title") { (visible_event_signal.get().name) }
				form(id="event_log_jump", on:submit=jump_handler) {
					input(type="text", bind:value=jump_id_entry, placeholder="ID")
					button(type="submit") { "Jump" }
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
					div(class="event_log_header") { }
					div(class="event_log_header") { }
					div(class="event_log_header") { }
					div(class="event_log_header") { "Editor" }
					div(class="event_log_header") { "Notes to editor" }
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
				view! {
					ctx,
					div(id="event_log_new_entry") {
						({
							if new_entry_typing_data.get().is_empty() {
								view! { ctx, }
							} else {
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
										div(class="event_log_header") { "Notes to editor" }
										EventLogEntryTyping(typing_data=new_entry_typing_data)
									}
								}
							}
						})
						EventLogEntryEdit(
							event=read_event_signal,
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
							make_video=new_entry_make_video,
							notes_to_editor=new_entry_notes_to_editor,
							editor=new_entry_editor,
							editor_name_index=editors_by_name_index,
							editor_name_datalist_id="editor_names",
							highlighted=new_entry_highlighted,
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
