use crate::color_utils::rgb_str_from_color;
use crate::pages::error::ErrorData;
use chrono::{DateTime, Duration, Utc};
use contrast::contrast;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use rgb::RGB8;
use std::collections::HashMap;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::event_subscription::{EventSubscriptionUpdate, NewTypingData};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::RequestMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore_router::navigate;
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

#[derive(Prop)]
pub struct EventLogEntryRowProps<THandler: Fn()> {
	entry: EventLogEntry,
	event: Event,
	entry_type: EntryType,
	click_handler: Option<THandler>,
}

#[component]
pub fn EventLogEntryRow<'a, G: Html, T: Fn() + 'a>(ctx: Scope<'a>, props: EventLogEntryRowProps<T>) -> View<G> {
	let start_time = props.entry.start_time - props.event.start_time;
	let start_time_display = format_duration(&start_time);
	let end_time_display = if let Some(entry_end_time) = props.entry.end_time {
		let end_time = entry_end_time - props.event.start_time;
		format_duration(&end_time)
	} else {
		String::new()
	};
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
	let tags_signal = create_signal(ctx, props.entry.tags.clone());

	let mut row_class = String::from("event_log_entry");
	if props.entry.highlighted {
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

	view! {
		ctx,
		div(class=row_class, on:click=click_handler) {
			div(class="log_entry_start_time") { (start_time_display) }
			div(class="log_entry_end_time") { (end_time_display) }
			div(class="log_entry_type", style=entry_type_style) { (props.entry_type.name) }
			div(class="log_entry_description") { (props.entry.description) }
			div(class="log_entry_submitter_winner") { (props.entry.submitter_or_winner) }
			div(class="log_entry_media_link") {
				(if !props.entry.media_link.is_empty() {
					let media_link = props.entry.media_link.clone();
					let media_link_link = media_link.clone();
					view! {
						ctx,
						a(href=media_link_link) {
							(media_link)
						}
					}
				} else {
					view! { ctx, }
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
				(if props.entry.make_video {
					view! {
						ctx,
						img(src="images/video.png")
					}
				} else {
					view! { ctx, }
				})
			}
			div(class="log_entry_editor_link") {
				(if let Some(link) = props.entry.editor_link.as_ref() {
					let link = link.clone();
					view! {
						ctx,
						a(href=link) { "Edit" }
					}
				} else {
					view! { ctx, }
				})
			}
			div(class="log_entry_video_link") {
				(if let Some(link) = props.entry.video_link.as_ref() {
					let link = link.clone();
					view! {
						ctx,
						a(href=link) { "Video" }
					}
				} else {
					view! { ctx, }
				})
			}
			div(class="log_entry_editor_user") {
				(if let Some(editor) = props.entry.editor.as_ref() {
					let name_color = rgb_str_from_color(editor.color);
					let name_style = format!("color: {}", name_color);
					let username = editor.username.clone();
					view! {
						ctx,
						span(style=name_style) { (username) }
					}
				} else {
					view! { ctx, }
				})
			}
			div(class="log_entry_notes_to_editor") {
				(props.entry.notes_to_editor)
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
	close_handler: TCloseHandler,
	editing_new: bool,
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
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = RequestMessage::EventSubscriptionUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::StartTime(
					(*props.event_log_entry.get()).clone(),
					(*start_time_input.get()).clone(),
				))),
			);
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize typing notification",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send typing notification",
					error,
				)));
				navigate("/error");
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
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = RequestMessage::EventSubscriptionUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::EndTime(
					(*props.event_log_entry.get()).clone(),
					(*end_time_input.get()).clone(),
				))),
			);
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize typing notification",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send typing notification",
					error,
				)));
				navigate("/error");
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
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = RequestMessage::EventSubscriptionUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::EntryType(
					(*props.event_log_entry.get()).clone(),
					(*entry_type_name.get()).clone(),
				))),
			);
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize typing notification",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send typing notification",
					error,
				)));
				navigate("/error");
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
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = RequestMessage::EventSubscriptionUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::Description(
					(*props.event_log_entry.get()).clone(),
					(*props.description.get()).clone(),
				))),
			);
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize typing notification",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send typing notification",
					error,
				)));
				navigate("/error");
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
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = RequestMessage::EventSubscriptionUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::MediaLink(
					(*props.event_log_entry.get()).clone(),
					(*props.media_link.get()).clone(),
				))),
			);
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize typing notification",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send typing notification",
					error,
				)));
				navigate("/error");
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
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = RequestMessage::EventSubscriptionUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::SubmitterWinner(
					(*props.event_log_entry.get()).clone(),
					(*props.submitter_or_winner.get()).clone(),
				))),
			);
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize typing notification",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send typing notification",
					error,
				)));
				navigate("/error");
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
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = RequestMessage::EventSubscriptionUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::NotesToEditor(
					(*props.event_log_entry.get()).clone(),
					(*props.notes_to_editor.get()).clone(),
				))),
			);
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize typing notification",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send typing notification",
					error,
				)));
				navigate("/error");
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

		if props.editing_new {
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

	let disable_save = create_memo(ctx, || {
		start_time_error.get().is_some()
			|| end_time_error.get().is_some()
			|| entry_type_error.get().is_some()
			|| editor_error.get().is_some()
			|| !new_tag_names.get().is_empty()
	});

	view! {
		ctx,
		form(class="event_log_entry_edit", on:submit=close_handler) {
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
												let ws_context: &Mutex<WebSocket> = use_context(ctx);
												let mut ws = ws_context.lock().await;
												let new_tag = Tag { id: String::new(), name: tag_name.clone(), description: (*description_signal.get()).clone() };
												let message = RequestMessage::EventSubscriptionUpdate((*props.event.get()).clone(), Box::new(EventSubscriptionUpdate::NewTag(new_tag)));
												let message_json = match serde_json::to_string(&message) {
													Ok(msg) => msg,
													Err(error) => {
														let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
														error_signal.set(Some(ErrorData::new_with_error("Failed to serialize new tag creation message", error)));
														navigate("/error");
														return;
													}
												};
												if let Err(error) = ws.send(Message::Text(message_json)).await {
													let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
													error_signal.set(Some(ErrorData::new_with_error("Failed to send new tag creation message", error)));
													navigate("/error");
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
					(if props.editing_new {
						view! {
							ctx,
							button(disabled=*disable_save.get()) { "Add" }
							button(type="reset") { "Reset" }
						}
					} else {
						view! {
							ctx,
							button(disabled=*disable_save.get()) { "Close" }
						}
					})
				}
			}
		}
	}
}
