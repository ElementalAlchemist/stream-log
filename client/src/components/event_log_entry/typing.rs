use super::UserTypingData;
use crate::color_utils::rgb_str_from_color;
use crate::subscriptions::event::TypingTarget;
use std::collections::HashMap;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

#[derive(Prop)]
pub struct EventLogEntryTypingProps<'a> {
	typing_data: &'a ReadSignal<HashMap<String, UserTypingData>>,
}

#[component]
pub fn EventLogEntryTyping<'a, G: Html>(ctx: Scope<'a>, props: EventLogEntryTypingProps<'a>) -> View<G> {
	let user_typing_data = create_memo(ctx, || {
		let typing_data: Vec<(UserData, [String; 7])> = props
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
				];
				for (target, value) in typing_data.iter() {
					let value_index = match *target {
						TypingTarget::StartTime => 0,
						TypingTarget::EndTime => 1,
						TypingTarget::EntryType => 2,
						TypingTarget::Description => 3,
						TypingTarget::SubmitterWinner => 4,
						TypingTarget::MediaLink => 5,
						TypingTarget::NotesToEditor => 6,
					};
					typing_values[value_index] = value.clone();
				}
				(user.clone(), typing_values)
			})
			.collect();
		typing_data
	});

	view! {
		ctx,
		Keyed(
			iterable=user_typing_data,
			key=|data| data.clone(),
			view=|ctx, (user, typing_events)| {
				let [typed_start_time, typed_end_time, typed_entry_type, typed_description, typed_submitter_or_winner, typed_media_link, typed_notes_to_editor] = typing_events;

				let user_color = rgb_str_from_color(user.color);
				let username_style = format!("color: {}", user_color);

				let username = user.username;

				view! {
					ctx,
					div(class="event_log_entry_typing_username", style=username_style) {
						(username)
					}
					div(class="event_log_entry_typing_data") {
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
						div {}
						div {}
						div {}
						div { (typed_notes_to_editor) }
						div {}
						div {}
					}
				}
			}
		)
	}
}
