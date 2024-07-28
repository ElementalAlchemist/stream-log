// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::utils::format_duration;
use super::UserTypingData;
use crate::color_utils::rgb_str_from_color;
use crate::subscriptions::event::TypingTarget;
use std::collections::HashMap;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::{EndTimeData, EventLogEntry};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

#[derive(Prop)]
pub struct EventLogEntryTypingProps<'a> {
	event: RcSignal<Event>,
	event_entry_types: &'a ReadSignal<Vec<EntryType>>,
	event_log: RcSignal<Vec<EventLogEntry>>,
	typing_data: &'a ReadSignal<HashMap<String, UserTypingData>>,
	use_editor_view: &'a ReadSignal<bool>,
}

type TypingStringData = (Option<String>, String, String, String, String, String, String, String);

#[component]
pub fn EventLogEntryTyping<'a, G: Html>(ctx: Scope<'a>, props: EventLogEntryTypingProps<'a>) -> View<G> {
	let user_typing_data = create_memo(ctx, || {
		let mut typing_data: Vec<(UserData, TypingStringData)> = props
			.typing_data
			.get()
			.values()
			.map(|(user, typing_data)| {
				let mut typing_values = (
					None,
					String::new(),
					String::new(),
					String::new(),
					String::new(),
					String::new(),
					String::new(),
					String::new(),
				);
				for (target, value) in typing_data.iter() {
					match *target {
						TypingTarget::Parent => typing_values.0 = Some(value.clone()),
						TypingTarget::StartTime => typing_values.1.clone_from(value),
						TypingTarget::EndTime => typing_values.2.clone_from(value),
						TypingTarget::EntryType => typing_values.3.clone_from(value),
						TypingTarget::Description => typing_values.4.clone_from(value),
						TypingTarget::SubmitterWinner => typing_values.5.clone_from(value),
						TypingTarget::MediaLink => typing_values.6.clone_from(value),
						TypingTarget::NotesToEditor => typing_values.7.clone_from(value),
					};
				}
				(user.clone(), typing_values)
			})
			.collect();
		typing_data.sort_unstable_by_key(|(user, _)| user.username.clone());
		typing_data
	});

	view! {
		ctx,
		Keyed(
			iterable=user_typing_data,
			key=|data| data.clone(),
			view=move |ctx, (user, typing_events)| {
				let (parent_id, typed_start_time, typed_end_time, typed_entry_type, typed_description, typed_submitter_or_winner, typed_media_link, typed_notes_to_editor) = typing_events;

				let user_color = rgb_str_from_color(user.color);
				let username_style = format!("color: {}", user_color);

				let username = user.username;

				let event = (*props.event.get()).clone();
				let event_entry_types = (*props.event_entry_types.get()).clone();

				let parent_entry_data = match parent_id {
					Some(parent_id) => {
						if parent_id.is_empty() {
							Some(String::new())
						} else {
							let event_log = props.event_log.get();
							let parent = event_log.iter().find(|entry| entry.id == parent_id);

							if let Some(parent) = parent {
								let event = event.clone();
								let start_time_duration = parent.start_time - event.start_time;
								let Some(entry_type) = event_entry_types.iter().find(|entry_type| entry_type.id == parent.entry_type) else { return view! { ctx, } };
								let entry_type_name = entry_type.name.clone();
								let description = parent.description.clone();

								let start_time = format_duration(&start_time_duration);
								let end_time = match parent.end_time {
									EndTimeData::Time(time) => {
										let duration = time - event.start_time;
										format_duration(&duration)
									}
									EndTimeData::NotEntered => String::new(),
									EndTimeData::NoTime => String::from("—")
								};

								Some(format!("{} / {} / {} / {}", start_time, end_time, entry_type_name, description))
							} else {
								None
							}
						}
					}
					None => None
				};

				view! {
					ctx,
					div(class="event_log_entry_typing_header") {
						div(class="event_log_entry_typing_username", style=username_style) {
							(username)
						}
						div(class="event_log_entry_typing_parent") {
							(if let Some(parent_entry_data) = parent_entry_data.as_ref() {
								let parent_entry_data = parent_entry_data.clone();
								view! {
									ctx,
									img(class="event_log_entry_edit_parent_child_indicator", src="images/child-indicator.png")
									(if parent_entry_data.is_empty() {
										view! {
											ctx,
											img(class="event_log_entry_edit_no_parent_indicator", src="images/remove.png")
										}
									} else {
										let parent_entry_data = parent_entry_data.clone();
										view! {
											ctx,
											(parent_entry_data)
										}
									})
								}
							} else {
								view! { ctx, }
							})
						}
					}
					div(class="event_log_entry_typing_data") {
						div {}
						div {}
						div { (typed_start_time) }
						div { (typed_end_time) }
						div { (typed_entry_type) }
						div { (typed_description) }
						div { (typed_submitter_or_winner) }
						div { (typed_media_link) }
						div {}
						div {}
						div {}
						(if *props.use_editor_view.get() {
							view! {
								ctx,
								div {}
							}
						} else {
							view! { ctx, }
						})
						div {}
						(if *props.use_editor_view.get() {
							view ! {
								ctx,
								div {}
							}
						} else {
							view! { ctx, }
						})
						div { (typed_notes_to_editor) }
						(if *props.use_editor_view.get() {
							view! {
								ctx,
								div {}
								div {}
							}
						} else {
							view! { ctx, }
						})
					}
				}
			}
		)
	}
}
