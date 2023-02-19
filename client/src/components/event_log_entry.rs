use crate::color_utils::rgb_str_from_color;
use chrono::Duration;
use contrast::contrast;
use rgb::RGB8;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::events::Event;
use sycamore::prelude::*;

const WHITE: RGB8 = RGB8::new(255, 255, 255);
const BLACK: RGB8 = RGB8::new(0, 0, 0);

/// Formats a [`Duration`] object as hours:minutes
fn format_duration(duration: &Duration) -> String {
	let hours = duration.num_hours();
	let minutes = duration.num_minutes() % 60;
	format!("{}:{:02}", hours, minutes)
}

#[derive(Prop)]
pub struct EventLogEntryRowProps {
	entry: EventLogEntry,
	event: Event,
	entry_type: EntryType,
	click_handler: Option<Box<dyn Fn()>>,
}

#[component]
pub fn EventLogEntryRow<G: Html>(ctx: Scope<'_>, props: EventLogEntryRowProps) -> View<G> {
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

	view! {
		ctx,
		div(class=row_class, on:click=move |_| if let Some(click_handler) = &props.click_handler { (*click_handler)() }) {
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
