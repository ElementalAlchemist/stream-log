use crate::color_utils::rgb_str_from_color;
use chrono::{DateTime, Duration, Utc};
use contrast::contrast;
use rgb::RGB8;
use std::collections::HashMap;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::event_subscription::TypingData;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;
use web_sys::{Event as WebEvent, HtmlButtonElement};

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
			div(class="log_entry_video") {
				(if props.entry.make_video {
					view! {
						ctx,
						img(src="images/video.png")
					}
				} else {
					view! { ctx, }
				})
			}
			div(class="log_entry_editor") {
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
			div(class="log_entry_video") {
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
	event_start: DateTime<Utc>,
	event_entry_types: &'a ReadSignal<Vec<EntryType>>,
	event_tags_name_index: &'a ReadSignal<HashMap<String, Tag>>,
	entry_types_datalist_id: &'a str,
	entry: &'a ReadSignal<EventLogEntry>,
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
	editor_list: &'a ReadSignal<Vec<UserData>>,
	editor_name_index: &'a ReadSignal<HashMap<String, UserData>>,
	editor_name_datalist_id: &'a str,
	highlighted: &'a Signal<bool>,
	save_handler: TCloseHandler,
	cancel_handler: TCloseHandler,
	save_label: &'static str,
	cancel_label: &'static str,
	persistent: bool,
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

	let initial_start_time_duration = *props.start_time.get() - props.event_start;
	let start_time_input = create_signal(ctx, format_duration(&initial_start_time_duration));
	let start_time_error: &Signal<Option<String>> = create_signal(ctx, None);
	create_effect(ctx, move || {
		let start_time_result = get_duration_from_formatted(&start_time_input.get());
		match start_time_result {
			Ok(duration) => {
				start_time_error.set(None);
				let new_start_time = props.event_start + duration;
				props.start_time.set(new_start_time);
			}
			Err(error) => start_time_error.set(Some(error)),
		}
	});

	let initial_end_time_duration = if let Some(end_time) = props.end_time.get().as_ref() {
		Some(*end_time - props.event_start)
	} else {
		None
	};
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
					let new_end_time = props.event_start + duration;
					props.end_time.set(Some(new_end_time));
				}
				Err(error) => end_time_error.set(Some(error)),
			}
		}
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

	let entered_tags: Vec<Option<Tag>> = props.tags.get().iter().map(|tag| Some(tag.clone())).collect();
	let entered_tags = create_signal(ctx, entered_tags);

	create_effect(ctx, || {
		let mut entered_tags = entered_tags.modify();
		if let Some(last_tag) = entered_tags.last() {
			if last_tag.is_none() {
				entered_tags.push(None);
			}
		} else {
			entered_tags.push(None);
		}
	});

	create_effect(ctx, || {
		let tags: Vec<Tag> = entered_tags.get().iter().flatten().cloned().collect();
		props.tags.set(tags);
	});

	let editor_entry = if let Some(editor) = (*props.editor.get()).as_ref() {
		editor.username.clone()
	} else {
		String::new()
	};
	let editor_entry = create_signal(ctx, editor_entry);
	let editor_error: &Signal<Option<String>> = create_signal(ctx, None);

	let save_button = create_node_ref(ctx);
	let cancel_button = create_node_ref(ctx);

	let save_close_handler = move |event: WebEvent| {
		event.prevent_default();
		(props.save_handler)();

		if !props.persistent {
			let save_button: DomNode = save_button.get();
			let save_button: HtmlButtonElement = save_button.unchecked_into();
			let cancel_button: DomNode = cancel_button.get();
			let cancel_button: HtmlButtonElement = cancel_button.unchecked_into();

			save_button.set_disabled(true);
			cancel_button.set_disabled(true);
		}
	};
	let cancel_close_handler = move |_event: WebEvent| {
		(props.cancel_handler)();

		if !props.persistent {
			let save_button: DomNode = save_button.get();
			let save_button: HtmlButtonElement = save_button.unchecked_into();
			let cancel_button: DomNode = cancel_button.get();
			let cancel_button: HtmlButtonElement = cancel_button.unchecked_into();

			save_button.set_disabled(true);
			cancel_button.set_disabled(true);
		}
	};

	view! {
		ctx,
		form(class="event_log_entry_edit", on:submit=save_close_handler) {
			div {
				input(placeholder="Start", bind:value=start_time_input, class=if start_time_error.get().is_some() { "error" } else { "" }, title=(*start_time_error.get()).as_ref().unwrap_or(&String::new()))
			}
			div {
				input(placeholder="End", bind:value=end_time_input, class=if end_time_error.get().is_some() { "error" } else { "" }, title=(*end_time_error.get()).as_ref().unwrap_or(&String::new()))
			}
			div {
				input(
					placeholder="Type",
					bind:value=entry_type_name,
					class=if entry_type_error.get().is_some() { "error" } else { "" },
					title=(*entry_type_error.get()).as_ref().unwrap_or(&String::new()),
					list=props.entry_types_datalist_id
				)
			}
			div {
				input(placeholder="Description", bind:value=props.description)
			}
			div {
				label {
					"Submitter/winner:"
					input(bind:value=props.submitter_or_winner)
				}
			}
			div {
				label { "Tags:" }
				Indexed(
					iterable=entered_tags,
					view=|ctx, entry| {
						let tag_name = if let Some(tag) = entry.as_ref() {
							tag.name.clone()
						} else {
							String::new()
						};
						let entry_signal = create_signal(ctx, tag_name);

						let error_signal: &Signal<Option<String>> = create_signal(ctx, None);

						view! {
							ctx,
							input(bind:value=entry_signal, class=if error_signal.get().is_some() { "error" } else { "" }, title=(*error_signal.get()).as_ref().unwrap_or(&String::new()))
						}
					}
				)
			}
			div {
				label {
					input(type="checkbox", bind:checked=props.make_video)
					"Should make video?"
				}
			}
			div {
				label {
					"Notes for editor:"
					input(bind:value=props.notes_to_editor)
				}
			}
			div {
				label {
					"Editor:"
					input(bind:value=editor_entry, list=props.editor_name_datalist_id, class=if editor_error.get().is_some() { "error" } else { "" }, title=(*editor_error.get()).as_ref().unwrap_or(&String::new()))
				}
			}
			div {
				button(ref=save_button) { (props.save_label) }
				button(type="button", on:click=cancel_close_handler, ref=cancel_button) { (props.cancel_label) }
			}
		}
	}
}
