// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::utils::format_duration;
use crate::color_utils::rgb_str_from_color;
use crate::entry_type_colors::use_white_foreground;
use crate::subscriptions::event::EventSubscriptionSignals;
use std::collections::HashMap;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::{EndTimeData, EventLogEntry, VideoEditState};
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Event as WebEvent, HtmlElement};

#[derive(Prop)]
pub struct EventLogEntryRowProps<'a> {
	entry: &'a ReadSignal<Option<EventLogEntry>>,
	event_subscription_data: EventSubscriptionSignals,
	can_edit: &'a ReadSignal<bool>,
	entry_type: &'a ReadSignal<Option<EntryType>>,
	jump_highlight_row_id: &'a Signal<String>,
	editing_log_entry: &'a Signal<Option<EventLogEntry>>,
	editing_entry_parent: &'a Signal<Option<EventLogEntry>>,
	child_depth: u32,
	entry_numbers: &'a ReadSignal<HashMap<String, usize>>,
	use_editor_view: &'a ReadSignal<bool>,
}

#[component]
pub fn EventLogEntryRow<'a, G: Html>(ctx: Scope<'a>, props: EventLogEntryRowProps<'a>) -> View<G> {
	let row_is_being_edited = create_memo(ctx, || {
		match ((*props.entry.get()).as_ref(), (*props.editing_log_entry.get()).as_ref()) {
			(Some(row_entry), Some(edit_entry)) => row_entry.id == edit_entry.id,
			_ => false,
		}
	});

	let mut child_indicators = Vec::new();
	let extend_width = props.child_depth.saturating_sub(1);
	for _ in 0..extend_width {
		child_indicators.push(view! { ctx, img(src="images/child-extension.png") });
	}
	if props.child_depth > 0 {
		child_indicators.push(view! { ctx, img(src="images/child-indicator.png") });
	}
	let child_indicators = View::new_fragment(child_indicators);

	let start_time = create_memo(ctx, {
		let event_start = props.event_subscription_data.event.get().start_time;
		move || {
			let Some(entry) = (*props.entry.get()).clone() else {
				return String::new();
			};
			let start_time_duration = entry.start_time - event_start;
			format_duration(&start_time_duration)
		}
	});

	let end_time = create_memo(ctx, {
		let event_start = props.event_subscription_data.event.get().start_time;
		move || {
			let Some(entry) = (*props.entry.get()).clone() else {
				return String::new();
			};
			match entry.end_time {
				EndTimeData::Time(time) => {
					let end_time_duration = time - event_start;
					format_duration(&end_time_duration)
				}
				EndTimeData::NotEntered => String::new(),
				EndTimeData::NoTime => String::from("—"),
			}
		}
	});

	let entry_type_style = create_memo(ctx, || {
		let Some(entry_type) = (*props.entry_type.get()).clone() else {
			return String::new();
		};

		let entry_type_background = rgb_str_from_color(entry_type.color);
		let entry_type_foreground = if use_white_foreground(&entry_type.color) {
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

	let media_links = create_memo(ctx, || {
		(*props.entry.get())
			.as_ref()
			.map(|entry| entry.media_links.clone())
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

	let is_secure_context = window().map(|window| window.is_secure_context()).unwrap_or(false);

	let row_is_visible = create_memo(ctx, {
		let video_edit_state_filters = props.event_subscription_data.video_edit_state_filters.clone();
		let video_processing_state_filters = props.event_subscription_data.video_processing_state_filters.clone();
		move || {
			let entry = props.entry.get();
			let video_edit_state_filters = video_edit_state_filters.get();
			let video_processing_state_filters = video_processing_state_filters.get();

			let entry = if let Some(entry) = entry.as_ref() {
				entry
			} else {
				return false;
			};

			(video_edit_state_filters.is_empty() || video_edit_state_filters.contains(&entry.video_edit_state))
				&& (video_processing_state_filters.is_empty()
					|| video_processing_state_filters.contains(&entry.video_processing_state))
		}
	});

	let prevent_row_click_handler = |event: WebEvent| {
		event.stop_propagation();
	};

	let parent_select_handler = move |event: WebEvent| {
		event.stop_propagation();
		props.editing_entry_parent.set((*props.entry.get()).clone());
	};

	view! {
		ctx,
		(if *row_is_visible.get() {
			let event = props.event_subscription_data.event.clone();

			let row_click_handler_for_id = move |focus_element_id: &str| {
				let focus_element_id = focus_element_id.to_string();
				move |_event: WebEvent| {
					if any_text_is_selected() {
						return;
					}
					let entry = (*props.entry.get()).clone();
					props.editing_log_entry.set(entry);
					props.jump_highlight_row_id.set(String::new());
					if !focus_element_id.is_empty() {
						if let Some(window) = window() {
							if let Some(document) = window.document() {
								if let Some(element) = document.get_element_by_id(&focus_element_id) {
									let html_element: HtmlElement = element.unchecked_into();
									let _ = html_element.focus();
								}
							}
						}
					}
				}
			};

			view! {
				ctx,
				div(
					id={
						if let Some(entry) = props.entry.get().as_ref() {
							format!("event_log_entry_{}", entry.id)
						} else {
							String::new()
						}
					},
					class="event_log_entry_top_border"
				)
				div(
					class={
						let mut row_class = String::from("event_log_entry");
						if (*props.entry.get())
							.as_ref()
							.map(|entry| entry.missing_giveaway_information)
							.unwrap_or(false)
						{
							row_class = format!("{} log_entry_missing_giveaway_highlight", row_class);
						} else if (*props.entry.get())
							.as_ref()
							.map(|entry| entry.end_time == EndTimeData::NotEntered)
							.unwrap_or(false)
						{
							row_class = format!("{} log_entry_end_highlight", row_class);
						}

						if *props.can_edit.get() {
							row_class = format!("{} click", row_class);
						}

						if (*props.entry.get())
							.as_ref()
							.map(|entry| entry.id == *props.jump_highlight_row_id.get())
							.unwrap_or(false)
						{
							row_class = format!("{} event_log_entry_jump_highlight", row_class);
						}

						if *row_is_being_edited.get() {
							row_class = format!("{} event_log_entry_edit_highlight", row_class);
						}

						row_class
					}
				) {
					div(class="log_entry_number") {
						({
							let entry_numbers = props.entry_numbers.get();
							let entry = props.entry.get();
							let entry = (*entry).as_ref();

							match entry {
								Some(entry) => match entry_numbers.get(&entry.id) {
									Some(num) => num.to_string(),
									None => String::new()
								}
								None => String::new()
							}
						})
					}
					div(class="log_entry_select_parent", on:click=prevent_row_click_handler) {
						(child_indicators)
						img(src="images/add.png", class="click", alt="Add child entry", title="Add child entry", on:click=parent_select_handler)
					}
					div(class="log_entry_start_time", on:click=row_click_handler_for_id("event_log_entry_edit_start_time_field")) { (start_time.get()) }
					div(class="log_entry_end_time", on:click=row_click_handler_for_id("event_log_entry_edit_end_time_field")) { (end_time.get()) }
					div(
						class="log_entry_type",
						style=entry_type_style.get(),
						title=entry_type_description.get(),
						on:click=row_click_handler_for_id("event_log_entry_edit_type_field")
					) {
						(entry_type_name.get())
					}
					div(class="log_entry_description", on:click=row_click_handler_for_id("event_log_entry_edit_description_field")) {
						((*props.entry.get()).as_ref().map(|entry| entry.description.clone()).unwrap_or_default())
					}
					div(class="log_entry_submitter_winner", on:click=row_click_handler_for_id("event_log_entry_edit_submitter_or_winner_field")) {
						((*props.entry.get()).as_ref().map(|entry| entry.submitter_or_winner.clone()).unwrap_or_default())
					}
					div(class="log_entry_media_link") {
						Keyed(
							iterable=media_links,
							key=|link| link.clone(),
							view=|ctx, link| {
								let link_link = link.clone();
								view! {
									ctx,
									a(href=link_link, target="_blank", rel="noopener") {
										(link)
									}
								}
							}
						)
					}
					div(class="log_entry_tags", on:click=row_click_handler_for_id("event_log_entry_edit_add_tag_button")) {
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
					div(class="log_entry_poster_moment", on:click=row_click_handler_for_id("event_log_entry_edit_poster_moment_checkbox")) {
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
						},
						on:click=row_click_handler_for_id("event_log_entry_edit_video_edit_state_first_button")
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
					(if *props.use_editor_view.get() {
						let editor_link_format = event.get().editor_link_format.clone();
						view! {
							ctx,
							div(class="log_entry_editor_link", on:click=prevent_row_click_handler) {
								({
									if let Some(entry) = (*props.entry.get()).as_ref() {
										let editor_link = editor_link_format.replace("{id}", &entry.id);
										if editor_link.is_empty() {
											view! { ctx, }
										} else {
											view! {
												ctx,
												a(href=editor_link, target="_blank", rel="noopener") {
													img(src="/images/edit.png", alt="Edit", title="Open editor")
												}
											}
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
					div(class="log_entry_video_link", on:click=prevent_row_click_handler) {
						({
							let video_link = (*props.entry.get()).as_ref().and_then(|entry| entry.video_link.clone());
							if let Some(link) = video_link.as_ref() {
								let link = link.clone();
								let copy_link = link.clone();
								view! {
									ctx,
									a(href=link, target="_blank", rel="noopener") {
										img(src="/images/youtube.png", alt="Video", title="Open video")
									}
									(if is_secure_context {
										let video_copy_click_handler = {
											let copy_link = copy_link.clone();
											move |_event: WebEvent| {
												let clipboard = if let Some(window) = window() {
													window.navigator().clipboard()
												} else {
													return;
												};
												// The JS Promise will handle itself, and we don't need to handle it here
												let _ = clipboard.write_text(&copy_link);
											}
										};
										view! {
											ctx,
											a(class="click", on:click=video_copy_click_handler) {
												img(src="/images/copy.png", alt="Copy Video Link", title="Copy link")
											}
										}
									} else {
										view! { ctx, }
									})
								}
							} else {
								view! { ctx, }
							}
						})
					}
					(if *props.use_editor_view.get() {
						view! {
							ctx,
							div(class="log_entry_editor_user", on:click=row_click_handler_for_id("event_log_entry_edit_editor_field")) {
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
						}
					} else {
						view! { ctx, }
					})
					div(class="log_entry_notes_to_editor", on:click=row_click_handler_for_id("event_log_entry_edit_notes_to_editor_field")) {
						((*props.entry.get()).as_ref().map(|entry| entry.notes_to_editor.clone()).unwrap_or_default())
					}
					(if *props.use_editor_view.get() {
						view! {
							ctx,
							div(class="log_entry_video_processing_state") {
								({
									let video_processing_state = (*props.entry.get()).as_ref().map(|entry| entry.video_processing_state).unwrap_or_default();
									format!("{}", video_processing_state)
								})
							}
							div(class="log_entry_video_errors") {
								({
									let video_errors = (*props.entry.get()).as_ref().map(|entry| entry.video_errors.clone());
									video_errors.unwrap_or_default()
								})
							}
						}
					} else {
						view! { ctx, }
					})
				}
			}
		} else {
			view! { ctx, }
		})
	}
}

/// Checks whether any text in the DOM is selected
fn any_text_is_selected() -> bool {
	if let Some(window) = window() {
		if let Ok(Some(selection)) = window.get_selection() {
			if selection.type_() == "Range" {
				return true;
			}
		}
	}
	false
}
