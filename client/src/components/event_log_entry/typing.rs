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

#[component]
pub fn EventLogEntryTyping<'a, G: Html>(ctx: Scope<'a>, props: EventLogEntryTypingProps<'a>) -> View<G> {
	let user_typing_data = create_memo(ctx, || {
		let mut typing_data: Vec<(UserData, [String; 8])> = props
			.typing_data
			.get()
			.values()
			.map(|(user, typing_data)| {
				let mut typing_values = [
					String::new(),
					String::new(),
					String::new(),
					String::new(),
					String::new(),
					String::new(),
					String::new(),
					String::new(),
				];
				for (target, value) in typing_data.iter() {
					let value_index = match *target {
						TypingTarget::Parent => 0,
						TypingTarget::StartTime => 1,
						TypingTarget::EndTime => 2,
						TypingTarget::EntryType => 3,
						TypingTarget::Description => 4,
						TypingTarget::SubmitterWinner => 5,
						TypingTarget::MediaLink => 6,
						TypingTarget::NotesToEditor => 7,
					};
					typing_values[value_index] = value.clone();
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
				let [parent_id, typed_start_time, typed_end_time, typed_entry_type, typed_description, typed_submitter_or_winner, typed_media_link, typed_notes_to_editor] = typing_events;

				let user_color = rgb_str_from_color(user.color);
				let username_style = format!("color: {}", user_color);

				let username = user.username;

				let parent_entry = if parent_id.is_empty() {
					None
				} else {
					props.event_log.get().iter().find(|entry| entry.id == parent_id).cloned()
				};
				let event = (*props.event.get()).clone();
				let event_entry_types = (*props.event_entry_types.get()).clone();

				view! {
					ctx,
					div(class="event_log_entry_typing_header") {
						div(class="event_log_entry_typing_username", style=username_style) {
							(username)
						}
						div(class="event_log_entry_typing_parent") {
							(if let Some(parent) = parent_entry.as_ref() {
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
								view! {
									ctx,
									img(class="event_log_entry_edit_parent_child_indicator", src="images/child-indicator.png")
									(start_time)
									"/"
									(end_time)
									"/"
									(entry_type_name)
									"/"
									(description)
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
