use super::utils::{format_duration, BLACK, WHITE};
use crate::color_utils::rgb_str_from_color;
use contrast::contrast;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::{EventLogEntry, VideoEditState};
use stream_log_shared::messages::events::Event;
use sycamore::prelude::*;
use web_sys::Event as WebEvent;

#[derive(Prop)]
pub struct EventLogEntryRowProps<'a, THandler: Fn()> {
	entry: &'a ReadSignal<Option<EventLogEntry>>,
	event: Event,
	entry_type: &'a ReadSignal<Option<EntryType>>,
	click_handler: Option<THandler>,
	jump_highlight_row_id: &'a Signal<String>,
	new_entry_parent: &'a Signal<Option<EventLogEntry>>,
	child_depth: u32,
}

#[component]
pub fn EventLogEntryRow<'a, G: Html, T: Fn() + 'a>(ctx: Scope<'a>, props: EventLogEntryRowProps<'a, T>) -> View<G> {
	let indent_pixels = 20 * props.child_depth;
	let select_parent_style = format!("margin-left: {}px", indent_pixels);

	let start_time = create_memo(ctx, {
		let event_start = props.event.start_time;
		move || {
			let Some(entry) = (*props.entry.get()).clone() else {
				return String::new();
			};
			let start_time_duration = entry.start_time - event_start;
			format_duration(&start_time_duration)
		}
	});

	let end_time = create_memo(ctx, {
		let event_start = props.event.start_time;
		move || {
			let Some(entry) = (*props.entry.get()).clone() else {
				return String::new();
			};
			let Some(entry_end_time) = entry.end_time else {
				let display_string = if entry.marked_incomplete {
					String::new()
				} else {
					String::from("—")
				};
				return display_string;
			};
			let end_time_duration = entry_end_time - event_start;
			format_duration(&end_time_duration)
		}
	});

	let entry_type_style = create_memo(ctx, || {
		let Some(entry_type) = (*props.entry_type.get()).clone() else {
			return String::new();
		};
		let entry_type_background = entry_type.color;

		let entry_type_light_contrast: f64 = contrast(entry_type_background, WHITE);
		let entry_type_dark_contrast: f64 = contrast(entry_type_background, BLACK);

		let entry_type_background = rgb_str_from_color(entry_type_background);
		let entry_type_foreground = if entry_type_light_contrast > entry_type_dark_contrast {
			"#ffffff"
		} else {
			"#000000"
		};

		format!(
			"background: {}; color: {}",
			entry_type_background, entry_type_foreground
		)
	});
	let entry_type_name = create_memo(ctx, || {
		(*props.entry_type.get())
			.as_ref()
			.map(|entry_type| entry_type.name.clone())
			.unwrap_or_default()
	});
	let entry_type_description = create_memo(ctx, || {
		(*props.entry_type.get())
			.as_ref()
			.map(|entry_type| entry_type.description.clone())
			.unwrap_or_default()
	});

	let tags_signal = create_signal(
		ctx,
		(*props.entry.get())
			.as_ref()
			.map(|entry| entry.tags.clone())
			.unwrap_or_default(),
	);
	create_effect(ctx, || {
		let entry = props.entry.get();
		if let Some(entry) = entry.as_ref() {
			tags_signal.set(entry.tags.clone());
		}
	});

	let has_click_handler = props.click_handler.is_some();

	let row_click_handler = move |_event: WebEvent| {
		if let Some(click_handler) = &props.click_handler {
			(*click_handler)();
		}
	};

	let parent_select_handler = move |event: WebEvent| {
		event.stop_propagation();
		props.new_entry_parent.set((*props.entry.get()).clone());
	};

	view! {
		ctx,
		div(class="event_log_entry_top_border")
		div(
			id={
				if let Some(entry) = props.entry.get().as_ref() {
					format!("event_log_entry_{}", entry.id)
				} else {
					String::new()
				}
			},
			class={
				let mut row_class = String::from("event_log_entry");
				if (*props.entry.get())
					.as_ref()
					.map(|entry| entry.marked_incomplete)
					.unwrap_or(false)
				{
					row_class = format!("{} log_entry_highlight", row_class);
				}

				if has_click_handler {
					row_class = format!("{} click", row_class);
				}

				if (*props.entry.get())
					.as_ref()
					.map(|entry| entry.id == *props.jump_highlight_row_id.get())
					.unwrap_or(false)
				{
					row_class = format!("{} event_log_entry_jump_highlight", row_class);
				}

				row_class
			},
			on:click=row_click_handler
		) {
			div(class="log_entry_select_parent", style=select_parent_style) {
				img(src="images/add.png", class="click", alt="Add child entry", title="Add child entry", on:click=parent_select_handler)
			}
			div(class="log_entry_start_time") { (start_time.get()) }
			div(class="log_entry_end_time") { (end_time.get()) }
			div(class="log_entry_type", style=entry_type_style.get(), title=entry_type_description.get()) { (entry_type_name.get()) }
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
							span(class="log_entry_tag", title=tag.description) { (tag.name) }
						}
					}
				)
			}
			div(class="log_entry_poster_moment") {
				(if (*props.entry.get()).as_ref().map(|entry| entry.poster_moment).unwrap_or_default() {
					"✔️"
				} else {
					""
				})
			}
			div(
				class={
					let mut classes = vec!["log_entry_video_edit_state"];
					let video_edit_state = (*props.entry.get()).as_ref().map(|entry| entry.video_edit_state).unwrap_or_default();
					match video_edit_state {
						VideoEditState::NoVideo => (),
						VideoEditState::MarkedForEditing => classes.push("log_entry_video_edit_state_marked"),
						VideoEditState::DoneEditing => classes.push("log_entry_video_edit_state_edited")
					}
					classes.join(" ")
				}
			) {
				({
					let video_edit_state = (*props.entry.get()).as_ref().map(|entry| entry.video_edit_state).unwrap_or_default();
					match video_edit_state {
						VideoEditState::NoVideo => view! { ctx, },
						VideoEditState::MarkedForEditing => {
							view! {
								ctx,
								span(title="A video should be created for this row") {
									"[+]"
								}
							}
						}
						VideoEditState::DoneEditing => {
							view! {
								ctx,
								span(title="A video has been edited for this row") {
									"[✔️]"
								}
							}
						}
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
			div(class="log_entry_video_state") {
				({
					let video_state = (*props.entry.get()).as_ref().and_then(|entry| entry.video_state);
					match video_state {
						Some(state) => format!("{}", state),
						None => String::new()
					}
				})
			}
			div(class="log_entry_video_errors") {
				({
					let video_errors = (*props.entry.get()).as_ref().map(|entry| entry.video_errors.clone());
					match video_errors {
						Some(errors) => errors,
						None => String::new()
					}
				})
			}
		}
	}
}
