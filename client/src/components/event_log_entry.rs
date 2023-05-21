use crate::color_utils::rgb_str_from_color;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::DataSignals;
use chrono::{DateTime, Duration, Utc};
use contrast::contrast;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use rgb::RGB8;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::event_subscription::{EventSubscriptionUpdate, NewTypingData};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::SubscriptionTargetUpdate;
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use web_sys::Event as WebEvent;

const WHITE: RGB8 = RGB8::new(255, 255, 255);
const BLACK: RGB8 = RGB8::new(0, 0, 0);

/// Formats a [`Duration`] object as hours:minutes
fn format_duration(duration: &Duration) -> String {
	let hours = duration.num_hours();
	let minutes = duration.num_minutes() % 60;
	format!("{}:{:02}", hours, minutes)
}

/// Parses a string formatted as hhh:mm into a [`Duration`] object. If parsing fails,
/// returns a string suitable for display to the user who entered the value.
fn get_duration_from_formatted(formatted_duration: &str) -> Result<Duration, String> {
	let Some((hours, minutes)) = formatted_duration.split_once(':') else {
		return Err(String::from("Invalid format"));
	};

	let hours: i64 = match hours.parse() {
		Ok(hours) => hours,
		Err(error) => return Err(format!("Couldn't parse hours: {}", error)),
	};

	let minutes: i64 = match minutes.parse() {
		Ok(mins) => mins,
		Err(error) => return Err(format!("Couldn't parse minutes: {}", error)),
	};

	let duration_minutes = hours * 60 + minutes;
	Ok(Duration::minutes(duration_minutes))
}

#[derive(Clone, Eq, Hash, PartialEq)]
enum ModifiedEventLogEntryParts {
	StartTime,
	EndTime,
	EntryType,
	Description,
	MediaLink,
	SubmitterOrWinner,
	Tags,
	MakeVideo,
	NotesToEditor,
	Editor,
	Highlighted,
}

#[derive(Prop)]
pub struct EventLogEntryProps<'a> {
	entry: EventLogEntry,
	event_signal: RcSignal<Event>,
	entry_types_signal: RcSignal<Vec<EntryType>>,
	all_log_entries: RcSignal<Vec<EventLogEntry>>,
	can_edit: &'a ReadSignal<bool>,
	tags_by_name_index: &'a ReadSignal<HashMap<String, Tag>>,
	editors_by_name_index: &'a ReadSignal<HashMap<String, UserData>>,
	read_event_signal: &'a ReadSignal<Event>,
	read_entry_types_signal: &'a ReadSignal<Vec<EntryType>>,
	new_entry_parent: &'a Signal<Option<EventLogEntry>>,
	entries_by_parent: &'a ReadSignal<HashMap<String, Vec<EventLogEntry>>>,
}

#[component]
pub fn EventLogEntry<'a, G: Html>(ctx: Scope<'a>, props: EventLogEntryProps<'a>) -> View<G> {
	let entry = props.entry;
	let can_edit = props.can_edit;
	let tags_by_name_index = props.tags_by_name_index;
	let editors_by_name_index = props.editors_by_name_index;
	let read_event_signal = props.read_event_signal;
	let read_entry_types_signal = props.read_entry_types_signal;

	let event_signal = props.event_signal.clone();
	let entry_types_signal = props.entry_types_signal.clone();
	let log_entries = props.all_log_entries.clone();

	let entry_types = entry_types_signal.get();
	let entry_type = (*entry_types).iter().find(|et| et.id == entry.entry_type).unwrap();
	let event = event_signal.get();
	let edit_open_signal = create_signal(ctx, false);
	let click_handler = if *can_edit.get() {
		Some(|| {
			edit_open_signal.set(true);
		})
	} else {
		None
	};
	let event_log_entry_signal = create_memo(ctx, {
		let log_entries = log_entries.clone();
		let entry_id = entry.id.clone();
		move || {
			log_entries
				.get()
				.iter()
				.find(|log_entry| log_entry.id == entry_id)
				.cloned()
		}
	});

	let child_log_entries = create_memo(ctx, || {
		let entries_by_parent = props.entries_by_parent.get();
		let event_log_entry = event_log_entry_signal.get();
		let Some(log_entry_id) = (*event_log_entry).as_ref().map(|entry| &entry.id) else { return Vec::new(); };
		entries_by_parent.get(log_entry_id).cloned().unwrap_or_default()
	});

	// Set up edit signals/data
	let edit_start_time = create_signal(ctx, entry.start_time);
	let edit_end_time = create_signal(ctx, entry.end_time);
	let edit_entry_type = create_signal(ctx, entry.entry_type.clone());
	let edit_description = create_signal(ctx, entry.description.clone());
	let edit_media_link = create_signal(ctx, entry.media_link.clone());
	let edit_submitter_or_winner = create_signal(ctx, entry.submitter_or_winner.clone());
	let edit_tags = create_signal(ctx, entry.tags.clone());
	let edit_make_video = create_signal(ctx, entry.make_video);
	let edit_notes_to_editor = create_signal(ctx, entry.notes_to_editor.clone());
	let edit_editor = create_signal(ctx, entry.editor.clone());
	let edit_highlighted = create_signal(ctx, entry.highlighted);

	let modified_data: &Signal<HashSet<ModifiedEventLogEntryParts>> = create_signal(ctx, HashSet::new());

	create_effect(ctx, || {
		if *edit_open_signal.get() {
			if let Some(entry) = event_log_entry_signal.get_untracked().as_ref() {
				edit_start_time.set(entry.start_time);
				edit_end_time.set(entry.end_time);
				edit_entry_type.set(entry.entry_type.clone());
				edit_description.set(entry.description.clone());
				edit_media_link.set(entry.media_link.clone());
				edit_submitter_or_winner.set(entry.submitter_or_winner.clone());
				edit_tags.set(entry.tags.clone());
				edit_make_video.set(entry.make_video);
				edit_notes_to_editor.set(entry.notes_to_editor.clone());
				edit_editor.set(entry.editor.clone());
				edit_highlighted.set(entry.highlighted);
			}
			modified_data.modify().clear();
		}
	});

	create_effect(ctx, || {
		edit_start_time.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::StartTime);
	});
	create_effect(ctx, || {
		edit_end_time.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::EndTime);
	});
	create_effect(ctx, || {
		edit_entry_type.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::EntryType);
	});
	create_effect(ctx, || {
		edit_description.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::Description);
	});
	create_effect(ctx, || {
		edit_media_link.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::MediaLink);
	});
	create_effect(ctx, || {
		edit_submitter_or_winner.track();
		modified_data
			.modify()
			.insert(ModifiedEventLogEntryParts::SubmitterOrWinner);
	});
	create_effect(ctx, || {
		edit_tags.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::Tags);
	});
	create_effect(ctx, || {
		edit_make_video.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::MakeVideo);
	});
	create_effect(ctx, || {
		edit_notes_to_editor.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::NotesToEditor);
	});
	create_effect(ctx, || {
		edit_editor.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::Editor);
	});
	create_effect(ctx, || {
		edit_highlighted.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::Highlighted);
	});

	let row_edit_parent: &Signal<Option<EventLogEntry>> = create_signal(ctx, None);

	let close_handler_entry = entry.clone();

	let child_event_signal = props.event_signal.clone();
	let child_entry_types_signal = props.entry_types_signal.clone();
	let child_all_log_entries_signal = props.all_log_entries.clone();

	view! {
		ctx,
		EventLogEntryRow(entry=event_log_entry_signal, event=(*event).clone(), entry_type=entry_type.clone(), click_handler=click_handler, new_entry_parent=props.new_entry_parent)
		(if *edit_open_signal.get() {
			let close_handler = {
				let entry = close_handler_entry.clone();
				let event_signal = event_signal.clone();
				let log_entries = log_entries.clone();
				move || {
					let entry = entry.clone();
					let event_signal = event_signal.clone();
					let log_entries = log_entries.clone();
					spawn_local_scoped(ctx, async move {
						edit_open_signal.set(false);

						let mut log_entries = log_entries.modify();
						let log_entry = log_entries.iter_mut().find(|log_entry| log_entry.id == entry.id);
						let log_entry = match log_entry {
							Some(entry) => entry,
							None => return
						};

						let event = (*event_signal.get()).clone();

						let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
						let mut ws = ws_context.lock().await;

						let modified_data = modified_data.modify();
						for changed_datum in modified_data.iter() {
							let event_message = match changed_datum {
								ModifiedEventLogEntryParts::StartTime => EventSubscriptionUpdate::ChangeStartTime(log_entry.clone(), *edit_start_time.get()),
								ModifiedEventLogEntryParts::EndTime => EventSubscriptionUpdate::ChangeEndTime(log_entry.clone(), *edit_end_time.get()),
								ModifiedEventLogEntryParts::EntryType => EventSubscriptionUpdate::ChangeEntryType(log_entry.clone(), (*edit_entry_type.get()).clone()),
								ModifiedEventLogEntryParts::Description => EventSubscriptionUpdate::ChangeDescription(log_entry.clone(), (*edit_description.get()).clone()),
								ModifiedEventLogEntryParts::MediaLink => EventSubscriptionUpdate::ChangeMediaLink(log_entry.clone(), (*edit_media_link.get()).clone()),
								ModifiedEventLogEntryParts::SubmitterOrWinner => EventSubscriptionUpdate::ChangeSubmitterWinner(log_entry.clone(), (*edit_submitter_or_winner.get()).clone()),
								ModifiedEventLogEntryParts::Tags => EventSubscriptionUpdate::ChangeTags(log_entry.clone(), (*edit_tags.get()).clone()),
								ModifiedEventLogEntryParts::MakeVideo => EventSubscriptionUpdate::ChangeMakeVideo(log_entry.clone(), *edit_make_video.get()),
								ModifiedEventLogEntryParts::NotesToEditor => EventSubscriptionUpdate::ChangeNotesToEditor(log_entry.clone(), (*edit_notes_to_editor.get()).clone()),
								ModifiedEventLogEntryParts::Editor => EventSubscriptionUpdate::ChangeEditor(log_entry.clone(), (*edit_editor.get()).clone()),
								ModifiedEventLogEntryParts::Highlighted => EventSubscriptionUpdate::ChangeHighlighted(log_entry.clone(), *edit_highlighted.get())
							};
							let event_message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(event.clone(), Box::new(event_message))));
							let event_message = match serde_json::to_string(&event_message) {
								Ok(msg) => msg,
								Err(error) => {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to serialize entry log change.", error));
									return;
								}
							};
							if let Err(error) = ws.send(Message::Text(event_message)).await {
								let data: &DataSignals = use_context(ctx);
								data.errors.modify().push(ErrorData::new_with_error("Failed to send log entry change.", error));
							}
						}
					});
				}
			};
			view! {
				ctx,
				EventLogEntryEdit(
					event=read_event_signal,
					event_entry_types=read_entry_types_signal,
					event_tags_name_index=tags_by_name_index,
					entry_types_datalist_id="event_entry_types",
					event_log_entry=event_log_entry_signal,
					tags_datalist_id="event_tags",
					start_time=edit_start_time,
					end_time=edit_end_time,
					entry_type=edit_entry_type,
					description=edit_description,
					media_link=edit_media_link,
					submitter_or_winner=edit_submitter_or_winner,
					tags=edit_tags,
					make_video=edit_make_video,
					notes_to_editor=edit_notes_to_editor,
					editor=edit_editor,
					editor_name_index=editors_by_name_index,
					editor_name_datalist_id="editor_names",
					highlighted=edit_highlighted,
					parent_log_entry=row_edit_parent,
					close_handler=close_handler
				)
			}
		} else {
			view! { ctx, }
		})
		div(class="event_log_entry_children") {
			Keyed(
				iterable=child_log_entries,
				key=|entry| entry.id.clone(),
				view={
					let event_signal = child_event_signal.clone();
					let entry_types_signal = child_entry_types_signal.clone();
					let all_log_entries = child_all_log_entries_signal.clone();
					move |ctx, entry| {
						let event_signal = event_signal.clone();
						let entry_types_signal = entry_types_signal.clone();
						let all_log_entries = all_log_entries.clone();
						view! {
							ctx,
							EventLogEntry(
								entry=entry,
								event_signal=event_signal,
								entry_types_signal=entry_types_signal,
								all_log_entries=all_log_entries,
								can_edit=can_edit,
								tags_by_name_index=props.tags_by_name_index,
								editors_by_name_index=props.editors_by_name_index,
								read_event_signal=props.read_event_signal,
								read_entry_types_signal=props.read_entry_types_signal,
								new_entry_parent=props.new_entry_parent,
								entries_by_parent=props.entries_by_parent
							)
						}
					}
				}
			)
		}
	}
}

#[derive(Prop)]
pub struct EventLogEntryRowProps<'a, THandler: Fn()> {
	entry: &'a ReadSignal<Option<EventLogEntry>>,
	event: Event,
	entry_type: EntryType,
	click_handler: Option<THandler>,
	new_entry_parent: &'a Signal<Option<EventLogEntry>>,
}

#[component]
pub fn EventLogEntryRow<'a, G: Html, T: Fn() + 'a>(ctx: Scope<'a>, props: EventLogEntryRowProps<'a, T>) -> View<G> {
	let start_time = (*props.entry.get())
		.as_ref()
		.map(|entry| entry.start_time - props.event.start_time);
	let start_time_display = start_time.as_ref().map(format_duration).unwrap_or_default();

	let end_time = (*props.entry.get())
		.as_ref()
		.and_then(|entry| entry.end_time.map(|end_time| end_time - props.event.start_time));
	let end_time_display = end_time.as_ref().map(format_duration).unwrap_or_default();

	let entry_type_background = props.entry_type.color;
	let entry_type_light_contrast: f64 = contrast(entry_type_background, WHITE);
	let entry_type_dark_contrast: f64 = contrast(entry_type_background, BLACK);
	let entry_type_background = rgb_str_from_color(entry_type_background);
	let entry_type_foreground = if entry_type_light_contrast > entry_type_dark_contrast {
		"#ffffff"
	} else {
		"#000000"
	};
	let entry_type_style = format!(
		"background: {}; color: {}",
		entry_type_background, entry_type_foreground
	);
	let tags_signal = create_signal(
		ctx,
		(*props.entry.get())
			.as_ref()
			.map(|entry| entry.tags.clone())
			.unwrap_or_default(),
	);

	let mut row_class = String::from("event_log_entry");
	if (*props.entry.get())
		.as_ref()
		.map(|entry| entry.highlighted)
		.unwrap_or(false)
	{
		row_class = format!("{} log_entry_highlight", row_class);
	}

	if props.click_handler.is_some() {
		row_class = format!("{} click", row_class);
	}

	let click_handler = move |_event: WebEvent| {
		if let Some(click_handler) = &props.click_handler {
			(*click_handler)();
		}
	};

	let parent_select_handler = move |_event: WebEvent| {
		props.new_entry_parent.set((*props.entry.get()).clone());
	};

	view! {
		ctx,
		div(class=row_class, on:click=click_handler) {
			div(class="log_entry_select_parent") {
				img(src="images/add.png", class="click", alt="Add child entry", title="Add child entry", on:click=parent_select_handler)
			}
			div(class="log_entry_start_time") { (start_time_display) }
			div(class="log_entry_end_time") { (end_time_display) }
			div(class="log_entry_type", style=entry_type_style) { (props.entry_type.name) }
			div(class="log_entry_description") { ((*props.entry.get()).as_ref().map(|entry| entry.description.clone()).unwrap_or_default()) }
			div(class="log_entry_submitter_winner") { ((*props.entry.get()).as_ref().map(|entry| entry.submitter_or_winner.clone()).unwrap_or_default()) }
			div(class="log_entry_media_link") {
				({
					let media_link = (*props.entry.get()).as_ref().map(|entry| entry.media_link.clone()).unwrap_or_default();
					if !media_link.is_empty() {
						let media_link_link = media_link.clone();
						view! {
							ctx,
							a(href=media_link_link) {
								(media_link)
							}
						}
					} else {
						view! { ctx, }
					}
				})
			}
			div(class="log_entry_tags") {
				Keyed(
					iterable=tags_signal,
					key=|tag| tag.id.clone(),
					view=|ctx, tag| {
						view! {
							ctx,
							span(class="log_entry_tag") { (tag.name) }
						}
					}
				)
			}
			div(class="log_entry_make_video") {
				({
					let make_video = (*props.entry.get()).as_ref().map(|entry| entry.make_video).unwrap_or(false);
					if make_video {
						view! {
							ctx,
							img(src="images/video.png", alt="A video should be created for this row")
						}
					} else {
						view! { ctx, }
					}
				})
			}
			div(class="log_entry_editor_link") {
				({
					let editor_link = (*props.entry.get()).as_ref().and_then(|entry| entry.editor_link.clone());
					if let Some(link) = editor_link.as_ref() {
						let link = link.clone();
						view! {
							ctx,
							a(href=link) { "Edit" }
						}
					} else {
						view! { ctx, }
					}
				})
			}
			div(class="log_entry_video_link") {
				({
					let video_link = (*props.entry.get()).as_ref().and_then(|entry| entry.video_link.clone());
					if let Some(link) = video_link.as_ref() {
						let link = link.clone();
						view! {
							ctx,
							a(href=link) { "Video" }
						}
					} else {
						view! { ctx, }
					}
				})
			}
			div(class="log_entry_editor_user") {
				({
					let editor = (*props.entry.get()).as_ref().and_then(|entry| entry.editor.clone());
					if let Some(editor) = editor.as_ref() {
						let name_color = rgb_str_from_color(editor.color);
						let name_style = format!("color: {}", name_color);
						let username = editor.username.clone();
						view! {
							ctx,
							span(style=name_style) { (username) }
						}
					} else {
						view! { ctx, }
					}
				})
			}
			div(class="log_entry_notes_to_editor") {
				((*props.entry.get()).as_ref().map(|entry| entry.notes_to_editor.clone()).unwrap_or_default())
			}
		}
	}
}

#[derive(Prop)]
pub struct EventLogEntryEditProps<'a, TCloseHandler: Fn()> {
	event: &'a ReadSignal<Event>,
	event_entry_types: &'a ReadSignal<Vec<EntryType>>,
	event_tags_name_index: &'a ReadSignal<HashMap<String, Tag>>,
	entry_types_datalist_id: &'a str,
	event_log_entry: &'a ReadSignal<Option<EventLogEntry>>,
	tags_datalist_id: &'a str,
	start_time: &'a Signal<DateTime<Utc>>,
	end_time: &'a Signal<Option<DateTime<Utc>>>,
	entry_type: &'a Signal<String>,
	description: &'a Signal<String>,
	media_link: &'a Signal<String>,
	submitter_or_winner: &'a Signal<String>,
	tags: &'a Signal<Vec<Tag>>,
	make_video: &'a Signal<bool>,
	notes_to_editor: &'a Signal<String>,
	editor: &'a Signal<Option<UserData>>,
	editor_name_index: &'a ReadSignal<HashMap<String, UserData>>,
	editor_name_datalist_id: &'a str,
	highlighted: &'a Signal<bool>,
	parent_log_entry: &'a Signal<Option<EventLogEntry>>,
	close_handler: TCloseHandler,
}

#[component]
pub fn EventLogEntryEdit<'a, G: Html, TCloseHandler: Fn() + 'a>(
	ctx: Scope<'a>,
	props: EventLogEntryEditProps<'a, TCloseHandler>,
) -> View<G> {
	let event_entry_types_name_index = create_memo(ctx, {
		let event_entry_types = (*props.event_entry_types.get()).clone();
		move || {
			let name_index: HashMap<String, EntryType> = event_entry_types
				.iter()
				.map(|entry_type| (entry_type.name.clone(), entry_type.clone()))
				.collect();
			name_index
		}
	});
	let event_entry_types_id_index = create_memo(ctx, {
		let event_entry_types = (*props.event_entry_types.get()).clone();
		move || {
			let id_index: HashMap<String, EntryType> = event_entry_types
				.iter()
				.map(|event_type| (event_type.id.clone(), event_type.clone()))
				.collect();
			id_index
		}
	});

	let event_start = props.event.get().start_time;
	let initial_start_time_duration = *props.start_time.get() - event_start;
	let start_time_input = create_signal(ctx, format_duration(&initial_start_time_duration));
	let start_time_error: &Signal<Option<String>> = create_signal(ctx, None);
	create_effect(ctx, move || {
		let start_time_result = get_duration_from_formatted(&start_time_input.get());
		match start_time_result {
			Ok(duration) => {
				start_time_error.set(None);
				let new_start_time = event_start + duration;
				props.start_time.set(new_start_time);
			}
			Err(error) => start_time_error.set(Some(error)),
		}
	});
	let start_time_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		start_time_input.track();
		if !*start_time_typing_ran_once.get_untracked() {
			start_time_typing_ran_once.set(true);
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::StartTime(
					(*props.event_log_entry.get()).clone(),
					(*start_time_input.get()).clone(),
				))),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize typing notification.",
						error,
					));
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send typing notification.", error));
			}
		});
	});

	let initial_end_time_duration = (*props.end_time.get()).as_ref().map(|end_time| *end_time - event_start);
	let initial_end_time_input = if let Some(duration) = initial_end_time_duration.as_ref() {
		format_duration(duration)
	} else {
		String::new()
	};
	let end_time_input = create_signal(ctx, initial_end_time_input);
	let end_time_error: &Signal<Option<String>> = create_signal(ctx, None);
	create_effect(ctx, move || {
		let end_time_input = &*end_time_input.get();
		if end_time_input.is_empty() {
			end_time_error.set(None);
			props.end_time.set(None);
		} else {
			let end_time_result = get_duration_from_formatted(end_time_input);
			match end_time_result {
				Ok(duration) => {
					end_time_error.set(None);
					let new_end_time = event_start + duration;
					props.end_time.set(Some(new_end_time));
				}
				Err(error) => end_time_error.set(Some(error)),
			}
		}
	});
	let end_time_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		end_time_input.track();
		if !*end_time_typing_ran_once.get_untracked() {
			end_time_typing_ran_once.set(true);
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::EndTime(
					(*props.event_log_entry.get()).clone(),
					(*end_time_input.get()).clone(),
				))),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize typing notification.",
						error,
					));
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send typing notification.", error));
			}
		});
	});

	let initial_entry_type_name =
		if let Some(entry_type) = event_entry_types_id_index.get().get(&*props.entry_type.get()) {
			entry_type.name.clone()
		} else {
			String::new()
		};
	let entry_type_name = create_signal(ctx, initial_entry_type_name);
	let entry_type_error: &Signal<Option<String>> = create_signal(ctx, None);
	create_effect(ctx, || {
		let name = entry_type_name.get();
		if name.is_empty() {
			entry_type_error.set(Some(String::from("An entry type is required")));
		} else if let Some(entry_type) = event_entry_types_name_index.get().get(&*name) {
			entry_type_error.set(None);
			props.entry_type.set(entry_type.id.clone());
		} else {
			entry_type_error.set(Some(String::from("No entry type exists with that name")));
		}
	});
	let entry_type_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		entry_type_name.track();
		if !*entry_type_typing_ran_once.get_untracked() {
			entry_type_typing_ran_once.set(true);
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::EntryType(
					(*props.event_log_entry.get()).clone(),
					(*entry_type_name.get()).clone(),
				))),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize typing notification.",
						error,
					));
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send typing notification.", error));
			}
		});
	});

	let description_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		props.description.track();
		if !*description_typing_ran_once.get_untracked() {
			description_typing_ran_once.set(true);
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::Description(
					(*props.event_log_entry.get()).clone(),
					(*props.description.get()).clone(),
				))),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize typing notification.",
						error,
					));
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send typing notification.", error));
			}
		});
	});

	let media_link_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		props.media_link.track();
		if !*media_link_typing_ran_once.get_untracked() {
			media_link_typing_ran_once.set(true);
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::MediaLink(
					(*props.event_log_entry.get()).clone(),
					(*props.media_link.get()).clone(),
				))),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize typing notification.",
						error,
					));
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send typing notification.", error));
			}
		});
	});

	let submitter_or_winner_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		props.submitter_or_winner.track();
		if !*submitter_or_winner_typing_ran_once.get_untracked() {
			submitter_or_winner_typing_ran_once.set(true);
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::SubmitterWinner(
					(*props.event_log_entry.get()).clone(),
					(*props.submitter_or_winner.get()).clone(),
				))),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize typing notification.",
						error,
					));
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send typing notification.", error));
			}
		});
	});

	let entered_tags: Vec<String> = props.tags.get().iter().map(|tag| tag.name.clone()).collect();
	let entered_tags = create_signal(ctx, entered_tags);
	let entered_tag_entry: &Signal<Vec<&Signal<String>>> = create_signal(ctx, Vec::new());

	create_effect(ctx, || {
		let mut tags: Vec<Tag> = Vec::new();
		for tag_name in entered_tags.get().iter() {
			if tag_name.is_empty() {
				continue;
			}
			if let Some(tag) = props.event_tags_name_index.get().get(tag_name) {
				tags.push(tag.clone());
			}
		}
		props.tags.set(tags);
	});

	create_effect(ctx, || {
		let tag_names = entered_tags.get();
		let last_entry = tag_names.last();
		if let Some(entry) = last_entry {
			if !entry.is_empty() {
				entered_tags.modify().push(String::new());
			}
		} else {
			entered_tags.modify().push(String::new());
		}
	});

	create_effect(ctx, move || {
		let mut tag_names_entry = entered_tag_entry.modify();
		for (tag_index, tag_name) in entered_tags.get().iter().enumerate() {
			if tag_names_entry.len() > tag_index {
				tag_names_entry[tag_index].set(tag_name.clone());
			} else {
				let tag_name_signal = create_signal(ctx, tag_name.clone());
				tag_names_entry.push(tag_name_signal);
				create_effect(ctx, move || {
					entered_tags.modify()[tag_index] = (*tag_name_signal.get()).clone();
				});
			}
		}
	});

	let new_tag_names = create_memo(ctx, || {
		let mut names: Vec<String> = Vec::new();
		props.event_tags_name_index.track();
		for tag_name in entered_tags.get().iter() {
			if !tag_name.is_empty() && props.event_tags_name_index.get().get(tag_name).is_none() {
				names.push(tag_name.clone());
			}
		}
		names
	});

	let notes_to_editor_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		props.notes_to_editor.track();
		if !*notes_to_editor_typing_ran_once.get_untracked() {
			notes_to_editor_typing_ran_once.set(true);
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::NotesToEditor(
					(*props.event_log_entry.get()).clone(),
					(*props.notes_to_editor.get()).clone(),
				))),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize typing notification.",
						error,
					));
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send typing notification.", error));
			}
		});
	});

	let editor_entry = if let Some(editor) = (*props.editor.get()).as_ref() {
		editor.username.clone()
	} else {
		String::new()
	};
	let editor_entry = create_signal(ctx, editor_entry);
	let editor_error: &Signal<Option<String>> = create_signal(ctx, None);
	create_effect(ctx, || {
		let editor_name = editor_entry.get();
		if editor_name.is_empty() {
			props.editor.set(None);
			return;
		}
		if let Some(editor_user) = props.editor_name_index.get().get(&*editor_name) {
			editor_error.set(None);
			props.editor.set(Some(editor_user.clone()));
		} else {
			editor_error.set(Some(String::from("The entered name couldn't be matched to an editor")));
		}
	});

	let close_handler = move |event: WebEvent| {
		event.prevent_default();
		(props.close_handler)();

		if props.event_log_entry.get().is_none() {
			let new_start_time = Utc::now() - event_start;
			let new_start_time_input = format_duration(&new_start_time);
			start_time_input.set(new_start_time_input);

			end_time_input.set(String::new());
			entry_type_name.set(String::new());
			props.description.set(String::new());
			props.media_link.set(String::new());
			props.submitter_or_winner.set(String::new());
			props.tags.set(Vec::new());
			props.make_video.set(false);
			props.notes_to_editor.set(String::new());
			editor_entry.set(String::new());
			props.highlighted.set(false);
		}
	};

	let delete_handler = move |_event: WebEvent| {
		let Some(log_entry) = (*props.event_log_entry.get()).clone() else { return; };
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::DeleteLogEntry(log_entry)),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data_signals: &DataSignals = use_context(ctx);
					data_signals.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize event log entry deletion.",
						error,
					));
					return;
				}
			};
			let send_result = ws.send(Message::Text(message_json)).await;
			if let Err(error) = send_result {
				let data_signals: &DataSignals = use_context(ctx);
				data_signals.errors.modify().push(ErrorData::new_with_error(
					"Failed to send event log entry deletion.",
					error,
				));
			}
		});
	};

	let disable_save = create_memo(ctx, || {
		start_time_error.get().is_some()
			|| end_time_error.get().is_some()
			|| entry_type_error.get().is_some()
			|| editor_error.get().is_some()
			|| !new_tag_names.get().is_empty()
	});

	let remove_parent_handler = |_event: WebEvent| {
		props.parent_log_entry.set(None);
	};

	view! {
		ctx,
		form(class="event_log_entry_edit", on:submit=close_handler) {
			div(class="event_log_entry_edit_parent_info") {
				(if let Some(parent) = props.parent_log_entry.get().as_ref() {
					let start_time_duration = parent.start_time - props.event.get().start_time;
					let end_time_duration = parent.end_time.map(|end_time| end_time - props.event.get().start_time);
					let event_entry_types = props.event_entry_types.get();
					let Some(entry_type) = event_entry_types.iter().find(|entry_type| entry_type.id == parent.entry_type) else { return view! { ctx, }};
					let entry_type_name = entry_type.name.clone();
					let description = parent.description.clone();

					let start_time = format_duration(&start_time_duration);
					let end_time = end_time_duration.map(|d| format_duration(&d)).unwrap_or_default();

					view! {
						ctx,
						img(class="event_log_entry_edit_parent_child_indicator", src="images/child-indicator.png")
						(start_time)
						" / "
						(end_time)
						" / "
						(entry_type_name)
						" / "
						(description)
						img(class="event_log_entry_edit_parent_remove click", src="images/remove.png", on:click=remove_parent_handler)
					}
				} else {
					view! { ctx, }
				})
			}
			div(class="event_log_entry_edit_basic_info") {
				div(class="event_log_entry_edit_start_time") {
					input(placeholder="Start", bind:value=start_time_input, class=if start_time_error.get().is_some() { "error" } else { "" }, title=(*start_time_error.get()).as_ref().unwrap_or(&String::new()))
				}
				div(class="event_log_entry_edit_end_time") {
					input(placeholder="End", bind:value=end_time_input, class=if end_time_error.get().is_some() { "error" } else { "" }, title=(*end_time_error.get()).as_ref().unwrap_or(&String::new()))
				}
				div(class="event_log_entry_edit_type") {
					input(
						placeholder="Type",
						bind:value=entry_type_name,
						class=if entry_type_error.get().is_some() { "error" } else { "" },
						title=(*entry_type_error.get()).as_ref().unwrap_or(&String::new()),
						list=props.entry_types_datalist_id
					)
				}
				div(class="event_log_entry_edit_description") {
					input(placeholder="Description", bind:value=props.description)
				}
				div(class="event_log_entry_edit_media_link") {
					input(bind:value=props.media_link, placeholder="Media link")
				}
				div(class="event_log_entry_edit_submitter_or_winner") {
					input(bind:value=props.submitter_or_winner, placeholder="Submitter/winner")
				}
			}
			div(class="event_log_entry_edit_tags") {
				label { "Tags:" }
				div(class="event_log_entry_edit_tags_fields") {
					Indexed(
						iterable=entered_tag_entry,
						view=move |ctx, entry_signal| {
							view! {
								ctx,
								div {
									input(bind:value=entry_signal, list=props.tags_datalist_id)
								}
							}
						}
					)
				}
			}
			div(class="event_log_entry_edit_new_tags") {
				(if new_tag_names.get().is_empty() {
					view! { ctx, }
				} else {
					view! {
						ctx,
						label { "New tags:" }
						div(class="event_log_entry_edit_new_tags_fields") {
							Indexed(
								iterable=new_tag_names,
								view=move |ctx, tag_name| {
									let description_signal = create_signal(ctx, String::new());
									let send_new_tag_creation = {
										let tag_name = tag_name.clone();
										move |event: WebEvent| {
											event.prevent_default();
											let tag_name = tag_name.clone();
											spawn_local_scoped(ctx, async move {
												let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
												let mut ws = ws_context.lock().await;
												let new_tag = Tag { id: String::new(), name: tag_name.clone(), description: (*description_signal.get()).clone() };
												let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate((*props.event.get()).clone(), Box::new(EventSubscriptionUpdate::NewTag(new_tag)))));
												let message_json = match serde_json::to_string(&message) {
													Ok(msg) => msg,
													Err(error) => {
														let data: &DataSignals = use_context(ctx);
														data.errors.modify().push(ErrorData::new_with_error("Failed to serialize new tag creation message.", error));
														return;
													}
												};
												if let Err(error) = ws.send(Message::Text(message_json)).await {
													let data: &DataSignals = use_context(ctx);
													data.errors.modify().push(ErrorData::new_with_error("Failed to send new tag creation message.", error));
												}
											});
										}
									};
									view! {
										ctx,
										form(on:submit=send_new_tag_creation, class="event_log_entry_edit_new_tags_create") {
											div { (tag_name) }
											div {
												input(bind:value=description_signal, placeholder="Describe this tag")
											}
											div {
												button { "Add Tag" }
											}
										}
									}
								}
							)
						}
					}
				})
			}
			div(class="event_log_entry_edit_misc_info") {
				div(class="event_log_entry_edit_make_video") {
					label {
						input(type="checkbox", bind:checked=props.make_video)
						"Should make video?"
					}
				}
				div(class="event_log_entry_edit_notes_to_editor") {
					input(bind:value=props.notes_to_editor, placeholder="Notes to editor")
				}
				div(class="event_log_entry_edit_editor") {
					input(bind:value=editor_entry, placeholder="Editor", list=props.editor_name_datalist_id, class=if editor_error.get().is_some() { "error" } else { "" }, title=(*editor_error.get()).as_ref().unwrap_or(&String::new()))
				}
				div(class="event_log_entry_edit_highlighted") {
					label {
						input(type="checkbox", bind:checked=props.highlighted)
						"Highlight row"
					}
				}
				div(class="event_log_entry_edit_close") {
					(if props.event_log_entry.get().is_none() {
						view! {
							ctx,
							button(disabled=*disable_save.get()) { "Add" }
							button(type="reset") { "Reset" }
						}
					} else {
						view! {
							ctx,
							button(type="button", on:click=delete_handler) { "Delete" }
							button(disabled=*disable_save.get()) { "Close" }
						}
					})
				}
			}
		}
	}
}
