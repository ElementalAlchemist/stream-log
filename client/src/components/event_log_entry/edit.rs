// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::utils::{format_duration, get_duration_from_formatted};
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::DataSignals;
use crate::websocket::WebSocketSendStream;
use chrono::Utc;
use futures::lock::Mutex;
use gloo_net::websocket::Message;
use std::collections::{BTreeMap, HashMap, HashSet};
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::{EndTimeData, EventLogEntry, EventLogTab, VideoEditState};
use stream_log_shared::messages::event_subscription::{
	EventSubscriptionUpdate, ModifiedEventLogEntryParts, NewTypingData,
};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::subscriptions::SubscriptionTargetUpdate;
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::{PublicUserData, SelfUserData};
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event as WebEvent, HtmlElement, KeyboardEvent};

#[derive(Prop)]
pub struct EventLogEntryEditProps<'a> {
	event: &'a ReadSignal<Event>,
	permission_level: &'a ReadSignal<PermissionLevel>,
	event_entry_types: &'a ReadSignal<Vec<EntryType>>,
	event_tags: &'a ReadSignal<Vec<Tag>>,
	event_editors: &'a ReadSignal<Vec<PublicUserData>>,
	event_log_tabs: &'a ReadSignal<Vec<EventLogTab>>,
	current_tab: &'a ReadSignal<Option<EventLogTab>>,
	event_log_entries: &'a ReadSignal<Vec<EventLogEntry>>,
	editing_log_entry: &'a Signal<Option<EventLogEntry>>,
	edit_parent_log_entry: &'a Signal<Option<EventLogEntry>>,
	save_message_queue: &'a Signal<Vec<FromClientMessage>>,
}

#[component]
pub fn EventLogEntryEdit<'a, G: Html>(ctx: Scope<'a>, props: EventLogEntryEditProps<'a>) -> View<G> {
	let editing_log_entry = create_memo(ctx, || (*props.editing_log_entry.get()).clone().unwrap_or_default());

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
	let event_entry_types_name_case_map = create_memo(ctx, || {
		let mut case_map: BTreeMap<String, String> = BTreeMap::new();
		for name in event_entry_types_name_index.get().keys() {
			case_map.insert(name.to_lowercase(), name.clone());
		}
		case_map
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
	let event_tags_name_index = create_memo(ctx, || {
		let tag_index: HashMap<String, Tag> = props
			.event_tags
			.get()
			.iter()
			.map(|tag| (tag.name.clone(), tag.clone()))
			.collect();
		tag_index
	});
	let event_editors_name_index = create_memo(ctx, || {
		let editor_index: HashMap<String, PublicUserData> = props
			.event_editors
			.get()
			.iter()
			.map(|editor| (editor.username.clone(), editor.clone()))
			.collect();
		editor_index
	});

	let modified_entry_data: &Signal<HashSet<ModifiedEventLogEntryParts>> = create_signal(ctx, HashSet::new());
	let suppress_typing_notifications = create_signal(ctx, true);

	create_effect(ctx, move || {
		let parent_entry = props.edit_parent_log_entry.get();
		if *suppress_typing_notifications.get_untracked() {
			return;
		}
		let parent_entry_id = (*parent_entry)
			.as_ref()
			.map(|entry| entry.id.clone())
			.unwrap_or_default();
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::Parent(
					(*editing_log_entry.get()).clone(),
					parent_entry_id,
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

			let send_result = ws.send(Message::Text(message_json)).await;
			if let Err(error) = send_result {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send typing notification.", error));
			}
		});
	});

	let start_time_warning_base = (*props.editing_log_entry.get())
		.as_ref()
		.and_then(|entry| entry.start_time);
	let start_time_warning_active = create_signal(ctx, false);
	let start_time_input = if let Some(entry) = props.editing_log_entry.get().as_ref() {
		if let Some(start_time) = entry.start_time {
			let initial_start_time_duration = start_time - props.event.get().start_time;
			format_duration(&initial_start_time_duration)
		} else {
			String::new()
		}
	} else {
		String::new()
	};
	let start_time_input = create_signal(ctx, start_time_input);
	let start_time_value = create_signal(
		ctx,
		if let Some(entry) = props.editing_log_entry.get().as_ref() {
			entry.start_time
		} else {
			None
		},
	);
	let start_time_error: &Signal<Option<String>> = create_signal(ctx, None);

	let initial_end_time = (*props.editing_log_entry.get())
		.as_ref()
		.map(|entry| entry.end_time)
		.unwrap_or(EndTimeData::NotEntered);
	let initial_end_time_duration = match initial_end_time {
		EndTimeData::Time(end_time) => Some(end_time - props.event.get().start_time),
		_ => None,
	};
	let initial_end_time_input = if let Some(duration) = initial_end_time_duration.as_ref() {
		format_duration(duration)
	} else {
		String::new()
	};
	let end_time_value = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.map(|entry| entry.end_time)
			.unwrap_or(EndTimeData::NotEntered),
	);
	let end_time_input = create_signal(ctx, initial_end_time_input);
	let end_time_error: &Signal<Option<String>> = create_signal(ctx, None);

	let initial_entry_type_id = (*props.editing_log_entry.get())
		.as_ref()
		.map(|entry| entry.entry_type.clone());
	let initial_entry_type_name = if let Some(Some(entry_type_id)) = initial_entry_type_id.as_ref() {
		if let Some(entry_type) = event_entry_types_id_index.get().get(entry_type_id) {
			entry_type.name.clone()
		} else {
			String::new()
		}
	} else {
		String::new()
	};
	let entry_type_id = create_signal(ctx, initial_entry_type_id.unwrap_or_default());
	let entry_type_name = create_signal(ctx, initial_entry_type_name);
	let entry_type_error: &Signal<Option<String>> = create_signal(ctx, None);

	let description = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.map(|entry| entry.description.clone())
			.unwrap_or_default(),
	);

	let submitter_or_winner = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.map(|entry| entry.submitter_or_winner.clone())
			.unwrap_or_default(),
	);

	let media_links = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.map(|entry| entry.media_links.clone())
			.unwrap_or_default(),
	);
	let media_links_with_index: &ReadSignal<Vec<(usize, String)>> =
		create_memo(ctx, || media_links.get().iter().cloned().enumerate().collect());

	let tags = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.map(|entry| entry.tags.clone())
			.unwrap_or_default(),
	);
	let tag_names = create_memo(ctx, || {
		let tag_names: Vec<String> = tags.get().iter().map(|tag| tag.name.clone()).collect();
		tag_names
	});
	let tag_names_with_index = create_memo(ctx, || {
		let tag_names_with_index: Vec<(usize, String)> = tag_names.get().iter().cloned().enumerate().collect();
		tag_names_with_index
	});

	let new_tag_names = create_memo(ctx, || {
		let mut names_with_index: Vec<String> = Vec::new();
		event_tags_name_index.track();
		for tag_name in tag_names.get().iter() {
			if !tag_name.is_empty() && !event_tags_name_index.get().contains_key(tag_name) {
				names_with_index.push(tag_name.clone());
			}
		}
		names_with_index
	});

	let video_edit_state = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.map(|entry| entry.video_edit_state)
			.unwrap_or_default(),
	);
	let video_edit_state_no_video = create_memo(ctx, || *video_edit_state.get() == VideoEditState::NoVideo);
	let video_edit_state_marked = create_memo(ctx, || *video_edit_state.get() == VideoEditState::MarkedForEditing);
	let video_edit_state_done = create_memo(ctx, || *video_edit_state.get() == VideoEditState::DoneEditing);
	let video_edit_state_set_no_video = |_event: WebEvent| {
		video_edit_state.set(VideoEditState::NoVideo);
		modified_entry_data
			.modify()
			.insert(ModifiedEventLogEntryParts::VideoEditState);
	};
	let video_edit_state_set_marked = |_event: WebEvent| {
		video_edit_state.set(VideoEditState::MarkedForEditing);
		modified_entry_data
			.modify()
			.insert(ModifiedEventLogEntryParts::VideoEditState);
	};
	let video_edit_state_set_done = |_event: WebEvent| {
		video_edit_state.set(VideoEditState::DoneEditing);
		modified_entry_data
			.modify()
			.insert(ModifiedEventLogEntryParts::VideoEditState);
	};

	let notes = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.map(|entry| entry.notes.clone())
			.unwrap_or_default(),
	);

	let editor_value = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.and_then(|entry| entry.editor.clone()),
	);
	let editor_entry = if let Some(editor) = (*editor_value.get()).as_ref() {
		editor.username.clone()
	} else {
		String::new()
	};
	let editor_entry = create_signal(ctx, editor_entry);
	let editor_error: &Signal<Option<String>> = create_signal(ctx, None);

	let poster_moment = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.map(|entry| entry.poster_moment)
			.unwrap_or_default(),
	);

	let missing_giveaway_information = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.map(|entry| entry.missing_giveaway_information)
			.unwrap_or_default(),
	);

	let disable_missing_giveaway_info = create_signal(ctx, false);

	let manual_sort_key = create_signal(
		ctx,
		(*props.editing_log_entry.get())
			.as_ref()
			.and_then(|entry| entry.manual_sort_key),
	);
	let sort_key_entry = create_signal(
		ctx,
		manual_sort_key.get().map(|key| key.to_string()).unwrap_or_default(),
	);

	create_effect(ctx, move || {
		let editing_log_entry = editing_log_entry.get();
		let start_time_input = start_time_input.get();
		let event_start = props.event.get().start_time;

		if start_time_input.is_empty() {
			if editing_log_entry.start_time.is_some() {
				start_time_error.set(Some(String::from(
					"Start time cannot be cleared from an entry that already has one",
				)));
			} else {
				start_time_error.set(None);
			}
			start_time_value.set(None);
			start_time_warning_active.set(false);

			modified_entry_data
				.modify()
				.insert(ModifiedEventLogEntryParts::StartTime);
		} else {
			let start_time_result = get_duration_from_formatted(&start_time_input);
			match start_time_result {
				Ok(duration) => {
					start_time_error.set(None);
					let new_start_time = event_start + duration;
					start_time_value.set(Some(new_start_time));

					let warning_start_time = start_time_warning_base.unwrap_or_else(Utc::now);
					start_time_warning_active.set((new_start_time - warning_start_time).num_minutes().abs() >= 60);

					modified_entry_data
						.modify()
						.insert(ModifiedEventLogEntryParts::StartTime);
				}
				Err(error) => start_time_error.set(Some(error)),
			}
		}
	});
	create_effect(ctx, move || {
		start_time_input.track();
		if *suppress_typing_notifications.get_untracked() {
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::StartTime(
					(*editing_log_entry.get()).clone(),
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

	create_effect(ctx, move || {
		let end_time_input = &*end_time_input.get();
		let entry_type_id = entry_type_id.get();
		let event_entry_types_id_index = event_entry_types_id_index.get();
		let event_start = props.event.get().start_time;
		if end_time_input.is_empty() {
			end_time_error.set(None);
			end_time_value.set(EndTimeData::NotEntered);
			modified_entry_data.modify().insert(ModifiedEventLogEntryParts::EndTime);
		} else if end_time_input.chars().all(|c| c == '-') {
			let entry_type = event_entry_types_id_index.get((*entry_type_id).as_deref().unwrap_or(""));
			if entry_type
				.map(|entry_type| entry_type.require_end_time)
				.unwrap_or(false)
			{
				end_time_error.set(Some(String::from("The selected entry type requires an end time")));
			} else {
				end_time_error.set(None);
				end_time_value.set(EndTimeData::NoTime);
				modified_entry_data.modify().insert(ModifiedEventLogEntryParts::EndTime);
			}
		} else {
			let end_time_result = get_duration_from_formatted(end_time_input);
			match end_time_result {
				Ok(duration) => {
					end_time_error.set(None);
					let new_end_time = event_start + duration;
					end_time_value.set(EndTimeData::Time(new_end_time));

					modified_entry_data.modify().insert(ModifiedEventLogEntryParts::EndTime);
				}
				Err(error) => end_time_error.set(Some(error)),
			}
		}
	});
	create_effect(ctx, move || {
		end_time_input.track();
		if *suppress_typing_notifications.get_untracked() {
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::EndTime(
					(*editing_log_entry.get()).clone(),
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

	create_effect(ctx, || {
		let name = entry_type_name.get();
		if name.is_empty() {
			entry_type_error.set(None);
			entry_type_id.set(None);
		} else if let Some(entry_type) = event_entry_types_name_index.get().get(&*name) {
			entry_type_error.set(None);
			entry_type_id.set(Some(entry_type.id.clone()));

			modified_entry_data
				.modify()
				.insert(ModifiedEventLogEntryParts::EntryType);
		} else {
			entry_type_error.set(Some(String::from("No entry type exists with that name")));
		}
	});
	create_effect(ctx, move || {
		entry_type_name.track();
		if *suppress_typing_notifications.get_untracked() {
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::EntryType(
					(*editing_log_entry.get()).clone(),
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

	create_effect(ctx, || {
		description.track();
		modified_entry_data
			.modify()
			.insert(ModifiedEventLogEntryParts::Description);
	});
	create_effect(ctx, move || {
		description.track();
		if *suppress_typing_notifications.get_untracked() {
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get_untracked()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::Description(
					(*editing_log_entry.get_untracked()).clone(),
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

	create_effect(ctx, || {
		media_links.track();
		modified_entry_data
			.modify()
			.insert(ModifiedEventLogEntryParts::MediaLinks);
	});
	create_effect(ctx, move || {
		let media_links = media_links.get().join("\n");
		if *suppress_typing_notifications.get_untracked() {
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::MediaLinks(
					(*editing_log_entry.get()).clone(),
					media_links,
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

	create_effect(ctx, || {
		submitter_or_winner.track();
		modified_entry_data
			.modify()
			.insert(ModifiedEventLogEntryParts::SubmitterOrWinner);
	});
	create_effect(ctx, move || {
		submitter_or_winner.track();
		if *suppress_typing_notifications.get_untracked() {
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::SubmitterWinner(
					(*editing_log_entry.get()).clone(),
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

	create_effect(ctx, || {
		media_links.track();
		modified_entry_data
			.modify()
			.insert(ModifiedEventLogEntryParts::MediaLinks);
	});

	create_effect(ctx, || {
		tags.track();
		modified_entry_data.modify().insert(ModifiedEventLogEntryParts::Tags);
	});

	create_effect(ctx, || {
		notes.track();
		modified_entry_data.modify().insert(ModifiedEventLogEntryParts::Notes);
	});
	create_effect(ctx, move || {
		notes.track();
		if *suppress_typing_notifications.get_untracked() {
			return;
		}
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::Notes(
					(*editing_log_entry.get()).clone(),
					(*notes.get()).clone(),
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

	create_effect(ctx, || {
		let editor_name = editor_entry.get();
		if editor_name.is_empty() {
			editor_value.set(None);
			editor_error.set(None);
			modified_entry_data.modify().insert(ModifiedEventLogEntryParts::Editor);
			return;
		}
		if let Some(editor_user) = event_editors_name_index.get().get(&*editor_name) {
			editor_error.set(None);
			editor_value.set(Some(editor_user.clone()));
			modified_entry_data.modify().insert(ModifiedEventLogEntryParts::Editor);
		} else {
			editor_error.set(Some(String::from("The entered name couldn't be matched to an editor")));
		}
	});

	create_effect(ctx, || {
		poster_moment.track();
		modified_entry_data
			.modify()
			.insert(ModifiedEventLogEntryParts::PosterMoment);
	});

	create_effect(ctx, || {
		missing_giveaway_information.track();
		modified_entry_data
			.modify()
			.insert(ModifiedEventLogEntryParts::MissingGiveawayInfo);
	});

	create_effect(ctx, || {
		let entered_end_time = end_time_input.get();
		let entered_submitter_or_winner = submitter_or_winner.get();
		let permission_level = props.permission_level.get();
		let entry_marked_missing_giveaway_info = missing_giveaway_information.get();
		let editing_existing_entry = props.editing_log_entry.get().is_some();

		if !entered_end_time.is_empty()
			&& !entered_end_time.chars().all(|c| c == '-')
			&& !entered_submitter_or_winner.is_empty()
		{
			missing_giveaway_information.set(false);
			disable_missing_giveaway_info.set(true);
		} else if editing_existing_entry
			&& *entry_marked_missing_giveaway_info
			&& *permission_level != PermissionLevel::Supervisor
		{
			disable_missing_giveaway_info.set(true);
		} else {
			disable_missing_giveaway_info.set(false);
		}
	});

	create_effect(ctx, || {
		let sort_key: Option<i32> = sort_key_entry.get().parse().ok();
		manual_sort_key.set(sort_key);
		modified_entry_data.modify().insert(ModifiedEventLogEntryParts::SortKey);
	});

	create_effect(ctx, || {
		props.edit_parent_log_entry.track();
		modified_entry_data.modify().insert(ModifiedEventLogEntryParts::Parent);
	});

	// After setting up all the effects, initialize the modified data tracking to empty
	modified_entry_data.modify().clear();
	suppress_typing_notifications.set(false);

	let insert_position_time = create_memo(ctx, || {
		let log_entries = props.event_log_entries.get();
		let editing_log_entry = props.editing_log_entry.get();
		let entered_start_time = start_time_value.get();
		let mut top_level_parent = if let Some(entry) = (*editing_log_entry).clone() {
			entry
		} else {
			return *entered_start_time;
		};

		if top_level_parent.parent.is_none() {
			// Because the top-level parent entry is the current entry, we want to use the updated start time
			return *entered_start_time;
		}

		while let Some(parent) = top_level_parent.parent.as_ref() {
			let parent_entry = log_entries.iter().find(|entry| entry.id == *parent);
			if let Some(entry) = parent_entry {
				top_level_parent = entry.clone();
			} else {
				break;
			}
		}

		top_level_parent.start_time
	});

	let insert_to_tab = create_memo(ctx, || {
		let tabs = props.event_log_tabs.get();
		let insert_time = insert_position_time.get();
		insert_time.map(|insert_time| {
			let mut insert_tab: Option<EventLogTab> = None;
			for tab in tabs.iter() {
				if insert_time >= tab.start_time {
					insert_tab = Some(tab.clone());
				} else {
					break;
				}
			}

			insert_tab
		})
	});

	let end_field_ref = create_node_ref(ctx);
	let type_field_ref = create_node_ref(ctx);

	let start_now = || {
		let start_time_duration = Utc::now() - props.event.get().start_time;
		let start_time_duration = format_duration(&start_time_duration);
		start_time_input.set(start_time_duration);
	};

	let start_now_handler = move |_event: WebEvent| {
		start_now();

		let end_field_node: DomNode = end_field_ref.get();
		let end_field: HtmlElement = end_field_node.unchecked_into();
		let _ = end_field.focus();
	};

	let end_now = || {
		let end_time_duration = Utc::now() - props.event.get().start_time;
		let end_time_duration = format_duration(&end_time_duration);
		end_time_input.set(end_time_duration);
	};

	let end_now_handler = move |_event: WebEvent| {
		end_now();

		let type_field_node: DomNode = type_field_ref.get();
		let type_field: HtmlElement = type_field_node.unchecked_into();
		let _ = type_field.focus();
	};

	let start_time_warning_confirmation = move |_event: WebEvent| {
		start_time_warning_active.set(false);
	};

	let entry_type_lost_focus = move |_event: WebEvent| {
		let entered_name = entry_type_name.get();
		let name_index = event_entry_types_name_index.get();
		let entered_type = name_index.get(&*entered_name);
		if entered_type.is_some() {
			return;
		}
		let lower_name = entered_name.to_lowercase();
		let case_map = event_entry_types_name_case_map.get();
		let entered_type_name = case_map.get(&lower_name);
		if let Some(name) = entered_type_name {
			entry_type_name.set(name.clone());
			return;
		}

		let mut found_name: Option<&String> = None;
		for (case_insensitive_name, case_sensitive_name) in case_map.range(lower_name.clone()..) {
			if !case_insensitive_name.starts_with(&lower_name) {
				break;
			}
			if found_name.is_some() {
				found_name = None;
				break;
			}
			found_name = Some(case_sensitive_name);
		}
		if let Some(name) = found_name {
			entry_type_name.set(name.clone());
		}
	};

	let add_media_link_handler = |_event: WebEvent| {
		media_links.modify().push(String::new());
	};

	let add_tag_handler = |_event: WebEvent| {
		tags.modify().push(Tag {
			id: String::new(),
			name: String::new(),
			description: String::new(),
			playlist: None,
		});
	};

	create_effect(ctx, move || {
		let editing_log_entry = props.editing_log_entry.get();
		suppress_typing_notifications.set(true);

		if let Some(entry) = editing_log_entry.as_ref() {
			let event_start_time = props.event.get_untracked().start_time;
			let start_duration = if let Some(start_time) = entry.start_time {
				let duration = start_time - event_start_time;
				format_duration(&duration)
			} else {
				String::new()
			};
			let end_duration = match entry.end_time {
				EndTimeData::Time(time) => {
					let duration = time - event_start_time;
					format_duration(&duration)
				}
				EndTimeData::NotEntered => String::new(),
				EndTimeData::NoTime => String::from("-"),
			};
			let entry_type = entry
				.entry_type
				.as_ref()
				.and_then(|entry_type| {
					event_entry_types_id_index
						.get()
						.get(entry_type)
						.map(|entry_type| entry_type.name.clone())
				})
				.unwrap_or_default();
			let parent_entry = entry.parent.as_ref().and_then(|parent_id| {
				props
					.event_log_entries
					.get_untracked()
					.iter()
					.find(|entry| entry.id == *parent_id)
					.cloned()
			});

			start_time_input.set(start_duration);
			end_time_input.set(end_duration);
			entry_type_name.set(entry_type);
			description.set(entry.description.clone());
			media_links.set(entry.media_links.clone());
			submitter_or_winner.set(entry.submitter_or_winner.clone());
			tags.set(entry.tags.clone());
			video_edit_state.set(entry.video_edit_state);
			notes.set(entry.notes.clone());
			editor_entry.set(
				entry
					.editor
					.as_ref()
					.map(|editor| editor.username.clone())
					.unwrap_or_default(),
			);
			missing_giveaway_information.set(entry.missing_giveaway_information);
			sort_key_entry.set(entry.manual_sort_key.map(|key| key.to_string()).unwrap_or_default());
			props.edit_parent_log_entry.set(parent_entry);
		} else {
			start_time_input.set(String::new());
			end_time_input.set(String::new());
			entry_type_name.set(String::new());
			description.set(String::new());
			media_links.set(Vec::new());
			submitter_or_winner.set(String::new());
			tags.set(Vec::new());
			video_edit_state.set(VideoEditState::default());
			notes.set(String::new());
			editor_entry.set(String::new());
			missing_giveaway_information.set(false);
			sort_key_entry.set(String::new());
			props.edit_parent_log_entry.set(None);
		}

		start_time_warning_active.set(false);
		modified_entry_data.modify().clear();
		suppress_typing_notifications.set(false);
	});

	let reset_data = move || {
		props.editing_log_entry.set(None);
	};

	let save_handler = move |event: WebEvent| {
		event.prevent_default();

		if let Some(entry) = (*props.editing_log_entry.get()).as_ref() {
			let mut entry = entry.clone();
			for modification in modified_entry_data.get().iter() {
				match *modification {
					ModifiedEventLogEntryParts::StartTime => entry.start_time = *start_time_value.get(),
					ModifiedEventLogEntryParts::EndTime => entry.end_time = *end_time_value.get(),
					ModifiedEventLogEntryParts::EntryType => entry.entry_type.clone_from(&(*entry_type_id.get())),
					ModifiedEventLogEntryParts::Description => entry.description.clone_from(&(*description.get())),
					ModifiedEventLogEntryParts::MediaLinks => {
						entry.media_links = (*media_links.get())
							.iter()
							.filter(|link| !link.is_empty())
							.cloned()
							.collect()
					}
					ModifiedEventLogEntryParts::SubmitterOrWinner => {
						entry.submitter_or_winner.clone_from(&(*submitter_or_winner.get()))
					}
					ModifiedEventLogEntryParts::Tags => {
						entry.tags = tags.get().iter().filter(|tag| !tag.name.is_empty()).cloned().collect()
					}
					ModifiedEventLogEntryParts::VideoEditState => entry.video_edit_state = *video_edit_state.get(),
					ModifiedEventLogEntryParts::PosterMoment => entry.poster_moment = *poster_moment.get(),
					ModifiedEventLogEntryParts::Notes => entry.notes.clone_from(&(*notes.get())),
					ModifiedEventLogEntryParts::Editor => entry.editor.clone_from(&(*editor_value.get())),
					ModifiedEventLogEntryParts::MissingGiveawayInfo => {
						entry.missing_giveaway_information = *missing_giveaway_information.get()
					}
					ModifiedEventLogEntryParts::SortKey => entry.manual_sort_key = *manual_sort_key.get(),
					ModifiedEventLogEntryParts::Parent => {
						entry.parent = (*props.edit_parent_log_entry.get())
							.as_ref()
							.map(|parent_entry| parent_entry.id.clone())
					}
				}
			}

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				(*props.event.get()).clone(),
				Box::new(EventSubscriptionUpdate::UpdateLogEntry(
					entry.clone(),
					modified_entry_data.get().iter().copied().collect(),
				)),
			)));

			props.save_message_queue.modify().push(message);
		}

		reset_data();
	};

	let cancel_handler = move |event: WebEvent| {
		event.prevent_default();

		let event = (*props.event.get()).clone();
		let editing_log_entry = (*editing_log_entry.get()).clone();
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				event,
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::Clear(editing_log_entry))),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize typing clear message.",
						error,
					));
					return;
				}
			};

			let send_result = ws.send(Message::Text(message_json)).await;
			if let Err(error) = send_result {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send typing clear message.", error));
			}
		});

		reset_data();
	};

	let delete_confirm_signal = create_signal(ctx, false);

	let delete_handler = move |_event: WebEvent| {
		delete_confirm_signal.set(true);
	};

	let delete_confirm_handler = move |_event: WebEvent| {
		let Some(log_entry) = (*props.editing_log_entry.get()).clone() else {
			return;
		};
		delete_confirm_signal.set(false);
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
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
		let event = (*props.event.get()).clone();
		let editing_log_entry = (*editing_log_entry.get()).clone();
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(
				event,
				Box::new(EventSubscriptionUpdate::Typing(NewTypingData::Clear(editing_log_entry))),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize typing clear message.",
						error,
					));
					return;
				}
			};

			let send_result = ws.send(Message::Text(message_json)).await;
			if let Err(error) = send_result {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send typing clear message.", error));
			}
		});

		reset_data();
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
		props.edit_parent_log_entry.set(None);
	};

	let key_handler = move |event: WebEvent| {
		let key_event: KeyboardEvent = event.unchecked_into();

		if !key_event.alt_key() || key_event.shift_key() || key_event.ctrl_key() || key_event.meta_key() {
			return;
		}

		match key_event.key().as_str() {
			"s" => {
				if props.editing_log_entry.get().is_none() {
					start_now();
				}
			}
			"e" => end_now(),
			"i" => {
				if !*disable_missing_giveaway_info.get() {
					missing_giveaway_information.set(!*missing_giveaway_information.get());
				}
			}
			_ => (),
		}
	};

	let user: &Signal<Option<SelfUserData>> = use_context(ctx);
	let use_spell_check = create_memo(ctx, move || {
		(*user.get()).as_ref().map(|user| user.use_spell_check).unwrap_or(false)
	});

	view! {
		ctx,
		datalist(id="event_log_entry_edit_type_list") {
			Keyed(
				iterable=props.event_entry_types,
				key=|entry_type| entry_type.id.clone(),
				view=|ctx, entry_type| {
					view! {
						ctx,
						option(value=entry_type.name)
					}
				}
			)
		}
		datalist(id="event_log_entry_edit_tags_list") {
			Keyed(
				iterable=props.event_tags,
				key=|tag| tag.id.clone(),
				view=|ctx, tag| {
					view! {
						ctx,
						option(value=tag.name)
					}
				}
			)
		}
		datalist(id="event_log_entry_edit_editors_list") {
			Keyed(
				iterable=props.event_editors,
				key=|editor| editor.id.clone(),
				view=|ctx, editor| {
					view! {
						ctx,
						option(value=editor.username)
					}
				}
			)
		}
		form(id="event_log_entry_edit", on:submit=save_handler, on:keydown=key_handler) {
			(if let Some(entry) = (*props.editing_log_entry.get()).as_ref() {
				let event_start_time = props.event.get().start_time;
				let start_duration = if let Some(start_time) = entry.start_time {
					let duration = start_time - event_start_time;
					format_duration(&duration)
				} else {
					String::new()
				};
				let end_duration = match entry.end_time {
					EndTimeData::Time(time) => {
						let duration = time - props.event.get().start_time;
						format_duration(&duration)
					}
					EndTimeData::NotEntered => String::new(),
					EndTimeData::NoTime => String::from("—")
				};
				let entry_type_id_index = event_entry_types_id_index.get();
				let entry_type = entry.entry_type.as_ref().and_then(|entry_type| entry_type_id_index.get(entry_type));
				let entry_type_name = entry_type.map(|entry_type| entry_type.name.clone()).unwrap_or_default();
				let header_text = format!("Editing entry: {} / {} / {} / {}", start_duration, end_duration, entry_type_name, entry.description);

				view! {
					ctx,
					div(id="event_log_entry_edit_editing_info", class="event_log_entry_edit_editing_info_existing") {
						(header_text)
					}
				}
			} else {
				view! {
					ctx,
					div(id="event_log_entry_edit_editing_info", class="event_log_entry_edit_editing_info_new") {
						"Creating new entry"
					}
				}
			})
			div(id="event_log_entry_edit_parent_info") {
				(if let Some(parent) = props.edit_parent_log_entry.get().as_ref() {
					let event_start_time = props.event.get().start_time;
					let event_entry_types = props.event_entry_types.get();
					let entry_type_name = parent.entry_type
						.as_ref()
						.and_then(|parent_entry_type| event_entry_types
							.iter()
							.find(|entry_type| entry_type.id == *parent_entry_type)
						)
						.map(|entry_type| entry_type.name.clone())
						.unwrap_or_default();
					let description = parent.description.clone();

					let start_time = if let Some(start_time) = parent.start_time {
						let start_time_duration = start_time - event_start_time;
						format_duration(&start_time_duration)
					} else {
						String::new()
					};
					let end_time = match parent.end_time {
						EndTimeData::Time(time) => {
							let duration = time - props.event.get().start_time;
							format_duration(&duration)
						}
						EndTimeData::NotEntered => String::new(),
						EndTimeData::NoTime => String::from("—")
					};

					view! {
						ctx,
						div {
							img(class="event_log_entry_edit_parent_child_indicator", src="images/child-indicator.png")
						}
						div {
							(start_time)
							" / "
							(end_time)
							" / "
							(entry_type_name)
							" / "
							(description)
						}
						div {
							img(id="event_log_entry_edit_parent_remove", class="click", src="images/remove.png", on:click=remove_parent_handler)
						}
					}
				} else {
					view! { ctx, }
				})
			}
			div(id="event_log_entry_edit_basic_info") {
				div(id="event_log_entry_edit_start_time") {
					input(
						placeholder="Start",
						bind:value=start_time_input,
						id="event_log_entry_edit_start_time_field",
						class=if start_time_error.get().is_some() { "error" } else { "" },
						title=(*start_time_error.get()).as_ref().unwrap_or(&String::new())
					)
					button(type="button", tabindex=-1, on:click=start_now_handler) { "Now" }
				}
				div(id="event_log_entry_edit_end_time") {
					input(
						placeholder="End",
						bind:value=end_time_input,
						id="event_log_entry_edit_end_time_field",
						class=if end_time_error.get().is_some() { "error" } else { "" },
						title=(*end_time_error.get()).as_ref().unwrap_or(&String::new()),
						ref=end_field_ref
					)
					button(type="button", tabindex=-1, on:click=end_now_handler) { "Now" }
				}
				div(id="event_log_entry_edit_type") {
					input(
						placeholder="Type",
						bind:value=entry_type_name,
						id="event_log_entry_edit_type_field",
						class=if entry_type_error.get().is_some() { "error" } else { "" },
						title=(*entry_type_error.get()).as_ref().unwrap_or(&String::new()),
						list="event_log_entry_edit_type_list",
						on:blur=entry_type_lost_focus,
						ref=type_field_ref
					)
				}
				div(id="event_log_entry_edit_description") {
					input(placeholder="Description", bind:value=description, id="event_log_entry_edit_description_field", spellcheck={use_spell_check.get()})
				}
				div(id="event_log_entry_edit_submitter_or_winner") {
					input(bind:value=submitter_or_winner, placeholder="Submitter/winner", id="event_log_entry_edit_submitter_or_winner_field")
				}
			}
			div(id="event_log_entry_edit_media_links") {
				label { "Media links:" }
				div(id="event_log_entry_edit_media_links_fields") {
					Keyed(
						iterable=media_links_with_index,
						key=|(index, _)| *index,
						view=move |ctx, (link_index, link)| {
							let link_entry = create_signal(ctx, link);
							create_effect(ctx, move || {
								let entered_link = link_entry.get();
								media_links.modify()[link_index].clone_from(&(*entered_link));
							});

							view! {
								ctx,
								div {
									input(bind:value=link_entry)
								}
							}
						}
					)
					div {
						button(type="button", on:click=add_media_link_handler) {
							"Add Link"
						}
					}
				}
			}
			div(id="event_log_entry_edit_tags") {
				label { "Tags:" }
				div(id="event_log_entry_edit_tags_fields") {
					Keyed(
						iterable=tag_names_with_index,
						key=|(index, _)| *index,
						view=move |ctx, (tag_index, tag_name)| {
							let tag_name_entry = create_signal(ctx, tag_name);
							let tag_description = create_memo(ctx, || {
								let tag_index = event_tags_name_index.get();
								tag_index.get(&*tag_name_entry.get()).map(|tag| tag.description.clone()).unwrap_or_default()
							});
							create_effect(ctx, move || {
								let tag_name = tag_name_entry.get();
								if let Some(tag) = tags.modify().get_mut(tag_index) {
									let existing_tag = event_tags_name_index.get().get(&*tag_name).cloned();
									let updated_tag = match existing_tag {
										Some(tag) => tag,
										None => Tag { id: String::new(), name: (*tag_name).clone(), description: String::new(), playlist: None }
									};
									*tag = updated_tag;
								}
							});
							view! {
								ctx,
								div {
									input(bind:value=tag_name_entry, list="event_log_entry_edit_tags_list", title=tag_description.get())
								}
							}
						}
					)
					div {
						button(type="button", id="event_log_entry_edit_add_tag_button", on:click=add_tag_handler) {
							"Add Tag"
						}
					}
				}
			}
			div(id="event_log_entry_edit_new_tags") {
				(if new_tag_names.get().is_empty() {
					view! { ctx, }
				} else {
					view! {
						ctx,
						label { "New tags:" }
						div(id="event_log_entry_edit_new_tags_fields") {
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
												let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
												let mut ws = ws_context.lock().await;
												let new_tag = Tag { id: String::new(), name: tag_name.clone(), description: (*description_signal.get()).clone(), playlist: None };
												let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate((*props.event.get()).clone(), Box::new(EventSubscriptionUpdate::UpdateTag(new_tag)))));
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
			div(id="event_log_entry_edit_misc_info") {
				div(id="event_log_entry_edit_video_edit_state") {
					button(
						type="button",
						class=if *video_edit_state_no_video.get() { "active_button_option" } else { "" },
						on:click=video_edit_state_set_no_video,
						id="event_log_entry_edit_video_edit_state_first_button"
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
				div(id="event_log_entry_edit_poster_moment") {
					label {
						input(type="checkbox", id="event_log_entry_edit_poster_moment_checkbox", bind:checked=poster_moment)
						"Poster moment"
					}
				}
				div(id="event_log_entry_edit_notes") {
					input(id="event_log_entry_edit_notes_field", bind:value=notes, placeholder="Notes", spellcheck={use_spell_check.get()})
				}
				div(id="event_log_entry_edit_editor") {
					input(
						bind:value=editor_entry,
						placeholder="Editor",
						list="event_log_entry_edit_editors_list",
						id="event_log_entry_edit_editor_field",
						class=if editor_error.get().is_some() { "error" } else { "" },
						title=(*editor_error.get()).as_ref().unwrap_or(&String::new())
					)
				}
				div(id="event_log_entry_edit_incomplete") {
					label {
						input(type="checkbox", bind:checked=missing_giveaway_information, disabled=*disable_missing_giveaway_info.get())
						"Needs giveaway results"
					}
				}
				div(id="event_log_entry_edit_sort_key") {
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
			div(id="event_log_entry_edit_close") {
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
					let insert_tab = insert_to_tab.get();
					if let Some(insert_tab) = (*insert_tab).as_ref() {
						if *insert_tab != *props.current_tab.get() {
							let insert_tab_name = (*insert_tab).as_ref().map(|tab| tab.name.clone()).unwrap_or_else(|| props.event.get().first_tab_name.clone());
							let display_error = format!("This entry will be added to a different tab: {}", insert_tab_name);
							view! {
								ctx,
								div(class="event_log_entry_edit_tab_warning") {
									(display_error)
								}
							}
						} else {
							view! { ctx, }
						}
					} else {
						view! { ctx, }
					}
				})
				(if let Some(entry) = (*props.editing_log_entry.get()).clone() {
					view! {
						ctx,
						div(id="event_log_entry_edit_delete") {
							(if entry.video_link.is_none() && *props.permission_level.get() == PermissionLevel::Supervisor {
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
						div(id="event_log_entry_id_info") {
							"ID: "
							(entry.id)
							({
								if entry.start_time.is_some() {
									let visible_creation_time = {
										let creation_duration = entry.created_at - props.event.get().start_time;
										format_duration(&creation_duration)
									};
									view! {
										ctx,
										" Created: "
										(visible_creation_time)
									}
								} else {
									view! { ctx, }
								}
							})
						}
						div(id="event_log_entry_edit_close_buttons") {
							button(disabled=*disable_save.get()) { "Save" }
							button(on:click=cancel_handler) { "Cancel" }
						}
					}
				} else {
					view! {
						ctx,
						div(id="event_log_entry_edit_delete")
						div(id="event_log_entry_edit_close_buttons") {
							button(disabled=*disable_save.get()) { "Add" }
							button(type="reset", on:click=reset_handler) { "Reset" }
						}
					}
				})
			}
		}
	}
}
