use super::utils::{format_duration, get_duration_from_formatted};
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::DataSignals;
use chrono::{DateTime, Utc};
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashMap;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::{EventLogEntry, VideoEditState};
use stream_log_shared::messages::event_subscription::{EventSubscriptionUpdate, NewTypingData};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::subscriptions::SubscriptionTargetUpdate;
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use web_sys::Event as WebEvent;

#[derive(Prop)]
pub struct EventLogEntryEditProps<'a, TCloseHandler: Fn(u8)> {
	event: &'a ReadSignal<Event>,
	permission_level: &'a ReadSignal<PermissionLevel>,
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
	video_edit_state: &'a Signal<VideoEditState>,
	poster_moment: &'a Signal<bool>,
	notes_to_editor: &'a Signal<String>,
	editor: &'a Signal<Option<UserData>>,
	editor_name_index: &'a ReadSignal<HashMap<String, UserData>>,
	editor_name_datalist_id: &'a str,
	marked_incomplete: &'a Signal<bool>,
	parent_log_entry: &'a Signal<Option<EventLogEntry>>,
	sort_key: &'a Signal<Option<i32>>,
	close_handler: TCloseHandler,
}

#[component]
pub fn EventLogEntryEdit<'a, G: Html, TCloseHandler: Fn(u8) + 'a>(
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
	let start_time_warning_base = (*props.event_log_entry.get()).as_ref().map(|entry| entry.start_time);
	let start_time_warning_active = create_signal(ctx, false);
	let start_time_input = if props.event_log_entry.get().is_some() {
		let initial_start_time_duration = *props.start_time.get() - event_start;
		create_signal(ctx, format_duration(&initial_start_time_duration))
	} else {
		create_signal(ctx, String::new())
	};
	let start_time_error: &Signal<Option<String>> = create_signal(ctx, None);
	create_effect(ctx, move || {
		let start_time_result = get_duration_from_formatted(&start_time_input.get());
		match start_time_result {
			Ok(duration) => {
				start_time_error.set(None);
				let new_start_time = event_start + duration;
				props.start_time.set(new_start_time);

				let warning_start_time = start_time_warning_base.unwrap_or_else(Utc::now);
				start_time_warning_active.set((new_start_time - warning_start_time).num_minutes().abs() >= 60);
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

	let description = create_signal(ctx, (*props.description.get()).clone());
	create_effect(ctx, || {
		props.description.set_rc(description.get());
	});
	let description_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		description.track();
		if !*description_typing_ran_once.get_untracked() {
			description_typing_ran_once.set(true);
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get_untracked()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::Description(
					(*props.event_log_entry.get_untracked()).clone(),
					(*description.get()).clone(),
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

	let media_link = create_signal(ctx, (*props.media_link.get()).clone());
	create_effect(ctx, || {
		props.media_link.set_rc(media_link.get());
	});
	let media_link_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		media_link.track();
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
					(*media_link.get()).clone(),
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

	let submitter_or_winner = create_signal(ctx, (*props.submitter_or_winner.get()).clone());
	create_effect(ctx, || {
		props.submitter_or_winner.set_rc(submitter_or_winner.get());
	});
	let submitter_or_winner_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		submitter_or_winner.track();
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
					(*submitter_or_winner.get()).clone(),
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

	let video_edit_state_no_video = create_memo(ctx, || *props.video_edit_state.get() == VideoEditState::NoVideo);
	let video_edit_state_marked = create_memo(ctx, || {
		*props.video_edit_state.get() == VideoEditState::MarkedForEditing
	});
	let video_edit_state_done = create_memo(ctx, || *props.video_edit_state.get() == VideoEditState::DoneEditing);
	let video_edit_state_set_no_video = |_event: WebEvent| {
		props.video_edit_state.set(VideoEditState::NoVideo);
	};
	let video_edit_state_set_marked = |_event: WebEvent| {
		props.video_edit_state.set(VideoEditState::MarkedForEditing);
	};
	let video_edit_state_set_done = |_event: WebEvent| {
		props.video_edit_state.set(VideoEditState::DoneEditing);
	};

	let notes_to_editor = create_signal(ctx, (*props.notes_to_editor.get()).clone());
	create_effect(ctx, || {
		props.notes_to_editor.set_rc(notes_to_editor.get());
	});
	let notes_to_editor_typing_ran_once = create_signal(ctx, false);
	create_effect(ctx, move || {
		notes_to_editor.track();
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
					(*notes_to_editor.get()).clone(),
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
			editor_error.set(None);
			return;
		}
		if let Some(editor_user) = props.editor_name_index.get().get(&*editor_name) {
			editor_error.set(None);
			props.editor.set(Some(editor_user.clone()));
		} else {
			editor_error.set(Some(String::from("The entered name couldn't be matched to an editor")));
		}
	});

	let disable_marked_incomplete = create_signal(ctx, false);
	create_effect(ctx, || {
		let entered_end_time = end_time_input.get();
		let entered_submitter_or_winner = submitter_or_winner.get();
		let permission_level = props.permission_level.get();
		let marked_incomplete = props.marked_incomplete.get();
		let editing_existing_entry = props.event_log_entry.get().is_some();

		if !entered_end_time.is_empty() && !entered_submitter_or_winner.is_empty() {
			props.marked_incomplete.set(false);
			disable_marked_incomplete.set(true);
		} else if editing_existing_entry && *marked_incomplete && *permission_level != PermissionLevel::Supervisor {
			disable_marked_incomplete.set(true);
		} else {
			disable_marked_incomplete.set(false);
		}
	});

	let sort_key_entry = create_signal(ctx, props.sort_key.get().map(|key| key.to_string()).unwrap_or_default());
	create_effect(ctx, || {
		let sort_key: Option<i32> = sort_key_entry.get().parse().ok();
		props.sort_key.set(sort_key);
	});

	let add_count_entry_signal = create_signal(ctx, String::from("1"));
	let add_count_signal = create_memo(ctx, || {
		let count: u8 = add_count_entry_signal.get().parse().unwrap_or(1);
		count
	});

	let start_now_handler = move |_event: WebEvent| {
		let start_time_duration = Utc::now() - event_start;
		let start_time_duration = format_duration(&start_time_duration);
		start_time_input.set(start_time_duration);
	};

	let end_now_handler = move |_event: WebEvent| {
		let end_time_duration = Utc::now() - event_start;
		let end_time_duration = format_duration(&end_time_duration);
		end_time_input.set(end_time_duration);
	};

	let start_time_warning_confirmation = move |_event: WebEvent| {
		start_time_warning_active.set(false);
	};

	let close_handler = move |event: WebEvent| {
		event.prevent_default();

		(props.close_handler)(*add_count_signal.get());
		if props.event_log_entry.get().is_none() {
			start_time_input.set(String::new());
			end_time_input.set(String::new());
			entry_type_name.set(String::new());
			description.set(String::new());
			media_link.set(String::new());
			submitter_or_winner.set(String::new());
			props.tags.set(Vec::new());
			props.video_edit_state.set(VideoEditState::default());
			notes_to_editor.set(String::new());
			editor_entry.set(String::new());
			props.marked_incomplete.set(false);
			props.parent_log_entry.set(None);
			entered_tag_entry.set(vec![create_signal(ctx, String::new())]);
			entered_tags.set(Vec::new());
			sort_key_entry.set(String::new());
			add_count_entry_signal.set(String::from("1"));
		}
	};

	let cancel_handler = |_event: WebEvent| {
		// We don't prevent event propagation here; this means that the form submit function will happen in addition to
		// (and after) this one.
		add_count_entry_signal.set(String::from("0"));
	};

	let delete_confirm_signal = create_signal(ctx, false);

	let delete_handler = move |_event: WebEvent| {
		delete_confirm_signal.set(true);
	};

	let delete_confirm_handler = move |_event: WebEvent| {
		let Some(log_entry) = (*props.event_log_entry.get()).clone() else {
			return;
		};
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

	let delete_cancel_handler = move |_event: WebEvent| {
		delete_confirm_signal.set(false);
	};

	let reset_handler = move |_event: WebEvent| {
		start_time_input.set(String::new());
		end_time_input.set(String::new());
		entry_type_name.set(String::new());
		description.set(String::new());
		media_link.set(String::new());
		submitter_or_winner.set(String::new());
		props.tags.set(Vec::new());
		props.video_edit_state.set(VideoEditState::default());
		notes_to_editor.set(String::new());
		editor_entry.set(String::new());
		props.marked_incomplete.set(false);
		sort_key_entry.set(String::new());
		add_count_entry_signal.set(String::from("1"));
	};

	let disable_save = create_memo(ctx, || {
		start_time_error.get().is_some()
			|| end_time_error.get().is_some()
			|| entry_type_error.get().is_some()
			|| editor_error.get().is_some()
			|| !new_tag_names.get().is_empty()
			|| *start_time_warning_active.get()
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
					input(
						placeholder="Start",
						bind:value=start_time_input,
						class=if start_time_error.get().is_some() { "error" } else { "" },
						title=(*start_time_error.get()).as_ref().unwrap_or(&String::new())
					)
					(
						if props.event_log_entry.get().is_none() {
							view! {
								ctx,
								button(type="button", tabindex=-1, on:click=start_now_handler) { "Now" }
							}
						} else {
							view! { ctx, }
						}
					)
				}
				div(class="event_log_entry_edit_end_time") {
					input(
						placeholder="End",
						bind:value=end_time_input,
						class=if end_time_error.get().is_some() { "error" } else { "" },
						title=(*end_time_error.get()).as_ref().unwrap_or(&String::new())
					)
					(
						if props.event_log_entry.get().is_none() {
							view! {
								ctx,
								button(type="button", tabindex=-1, on:click=end_now_handler) { "Now" }
							}
						} else {
							view! { ctx, }
						}
					)
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
					input(placeholder="Description", bind:value=description)
				}
				div(class="event_log_entry_edit_submitter_or_winner") {
					input(bind:value=submitter_or_winner, placeholder="Submitter/winner")
				}
				div(class="event_log_entry_edit_media_link") {
					input(bind:value=media_link, placeholder="Media link")
				}
			}
			div(class="event_log_entry_edit_tags") {
				label { "Tags:" }
				div(class="event_log_entry_edit_tags_fields") {
					Indexed(
						iterable=entered_tag_entry,
						view=move |ctx, entry_signal| {
							let tag_description = create_memo(ctx, || {
								let tag_index = props.event_tags_name_index.get();
								tag_index.get(&*entry_signal.get()).map(|tag| tag.description.clone()).unwrap_or_default()
							});
							view! {
								ctx,
								div {
									input(bind:value=entry_signal, list=props.tags_datalist_id, title=tag_description.get())
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
												let new_tag = Tag { id: String::new(), name: tag_name.clone(), description: (*description_signal.get()).clone(), playlist: String::new() };
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
				div(class="event_log_entry_edit_video_edit_state") {
					button(
						type="button",
						class=if *video_edit_state_no_video.get() { "active_button_option" } else { "" },
						on:click=video_edit_state_set_no_video
					) {
						"No Video"
					}
					button(
						type="button",
						class=if *video_edit_state_marked.get() { "active_button_option" } else { "" },
						on:click=video_edit_state_set_marked
					) {
						"Marked"
					}
					button(
						type="button",
						class=if *video_edit_state_done.get() { "active_button_option" } else { "" },
						on:click=video_edit_state_set_done
					) {
						"Done Editing"
					}
				}
				div(class="event_log_entry_edit_poster_moment") {
					label {
						input(type="checkbox", bind:checked=props.poster_moment)
						"Poster moment"
					}
				}
				div(class="event_log_entry_edit_notes_to_editor") {
					input(bind:value=notes_to_editor, placeholder="Notes to editor")
				}
				div(class="event_log_entry_edit_editor") {
					input(
						bind:value=editor_entry,
						placeholder="Editor",
						list=props.editor_name_datalist_id,
						class=if editor_error.get().is_some() { "error" } else { "" },
						title=(*editor_error.get()).as_ref().unwrap_or(&String::new())
					)
				}
				div(class="event_log_entry_edit_incomplete") {
					label {
						input(type="checkbox", bind:checked=props.marked_incomplete, disabled=*disable_marked_incomplete.get())
						"Mark incomplete"
					}
				}
				div(class="event_log_entry_edit_sort_key") {
					input(
						bind:value=sort_key_entry,
						placeholder="Sort",
						type="number",
						min=i32::MIN,
						max=i32::MAX,
						step=1
					)
				}
			}
			div(class="event_log_entry_edit_close") {
				(if *start_time_warning_active.get() {
					view! {
						ctx,
						div(class="event_log_entry_edit_start_warning") {
							"The entered start time was more than one hour out."
							button(type="button", on:click=start_time_warning_confirmation) {
								"It's correct"
							}
						}
					}
				} else {
					view! { ctx, }
				})
				(if let Some(entry) = (*props.event_log_entry.get()).clone() {
					view! {
						ctx,
						div(class="event_log_entry_edit_delete") {
							(if entry.video_link.is_none() {
								if *delete_confirm_signal.get() {
									view! {
										ctx,
										"This will really delete this row. Are you sure?"
										button(type="button", on:click=delete_confirm_handler) { "Yes, delete it!" }
										button(type="button", on:click=delete_cancel_handler) { "No, keep it!" }
									}
								} else {
									view! {
										ctx,
										button(type="button", on:click=delete_handler) { "Delete" }
									}
								}
							} else {
								view! { ctx, }
							})
						}
						div(class="event_log_entry_id") {
							"ID: "
							(entry.id)
						}
						div(class="event_log_entry_edit_close_buttons") {
							button(disabled=*disable_save.get()) { "Save" }
							button(on:click=cancel_handler) { "Cancel" }
						}
					}
				} else {
					view! {
						ctx,
						div(class="event_log_entry_edit_delete")
						div(class="event_log_entry_edit_add_multi") {
							"Add "
							input(type="number", min=1, max=u32::MAX, step=1, bind:value=add_count_entry_signal, class="event_log_entry_edit_add_count")
							" rows"
						}
						div(class="event_log_entry_edit_close_buttons") {
							button(disabled=*disable_save.get()) { "Add" }
							button(type="reset", on:click=reset_handler) { "Reset" }
						}
					}
				})
			}
		}
	}
}
