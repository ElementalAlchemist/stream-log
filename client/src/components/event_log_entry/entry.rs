use super::edit::EventLogEntryEdit;
use super::row::EventLogEntryRow;
use super::typing::EventLogEntryTyping;
use super::UserTypingData;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::event::TypingEvent;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::event_subscription::EventSubscriptionUpdate;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::subscriptions::SubscriptionTargetUpdate;
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum ModifiedEventLogEntryParts {
	StartTime,
	EndTime,
	EntryType,
	Description,
	MediaLink,
	SubmitterOrWinner,
	Tags,
	VideoEditState,
	PosterMoment,
	NotesToEditor,
	Editor,
	MarkedIncomplete,
	SortKey,
}

#[derive(Prop)]
pub struct EventLogEntryProps<'a> {
	entry: EventLogEntry,
	jump_highlight_row_id: &'a Signal<String>,
	event_signal: RcSignal<Event>,
	permission_level: &'a ReadSignal<PermissionLevel>,
	entry_types_signal: RcSignal<Vec<EntryType>>,
	all_log_entries: RcSignal<Vec<EventLogEntry>>,
	event_typing_events_signal: RcSignal<Vec<TypingEvent>>,
	can_edit: &'a ReadSignal<bool>,
	tags_by_name_index: &'a ReadSignal<HashMap<String, Tag>>,
	editors_by_name_index: &'a ReadSignal<HashMap<String, UserData>>,
	read_event_signal: &'a ReadSignal<Event>,
	read_entry_types_signal: &'a ReadSignal<Vec<EntryType>>,
	new_entry_parent: &'a Signal<Option<EventLogEntry>>,
	entries_by_parent: &'a ReadSignal<HashMap<String, Vec<EventLogEntry>>>,
	child_depth: u32,
}

#[component]
pub fn EventLogEntry<'a, G: Html>(ctx: Scope<'a>, props: EventLogEntryProps<'a>) -> View<G> {
	let entry = props.entry;
	let can_edit = props.can_edit;
	let tags_by_name_index = props.tags_by_name_index;
	let editors_by_name_index = props.editors_by_name_index;
	let read_event_signal = props.read_event_signal;
	let read_entry_types_signal = props.read_entry_types_signal;

	let event_signal = props.event_signal.clone();
	let entry_types_signal = props.entry_types_signal.clone();
	let log_entries = props.all_log_entries.clone();

	let event = event_signal.get();
	let edit_open_signal = create_signal(ctx, false);
	let click_handler = if *can_edit.get() {
		Some(|| {
			edit_open_signal.set(true);
			props.jump_highlight_row_id.set(String::new());
		})
	} else {
		None
	};
	let event_log_entry_signal = create_memo(ctx, {
		let log_entries = log_entries.clone();
		let entry_id = entry.id.clone();
		move || {
			log_entries
				.get()
				.iter()
				.find(|log_entry| log_entry.id == entry_id)
				.cloned()
		}
	});

	let entry_type = create_memo(ctx, move || {
		let entry = event_log_entry_signal.get();
		let entry_types = entry_types_signal.get();
		if let Some(entry) = entry.as_ref() {
			entry_types.iter().find(|et| et.id == entry.entry_type).cloned()
		} else {
			None
		}
	});

	let child_log_entries = create_memo(ctx, || {
		let entries_by_parent = props.entries_by_parent.get();
		let event_log_entry = event_log_entry_signal.get();
		let Some(log_entry_id) = (*event_log_entry).as_ref().map(|entry| &entry.id) else {
			return Vec::new();
		};
		entries_by_parent.get(log_entry_id).cloned().unwrap_or_default()
	});

	// Set up edit signals/data
	let edit_start_time = create_signal(ctx, entry.start_time);
	let edit_end_time = create_signal(ctx, entry.end_time);
	let edit_entry_type = create_signal(ctx, entry.entry_type.clone());
	let edit_description = create_signal(ctx, entry.description.clone());
	let edit_media_link = create_signal(ctx, entry.media_link.clone());
	let edit_submitter_or_winner = create_signal(ctx, entry.submitter_or_winner.clone());
	let edit_tags = create_signal(ctx, entry.tags.clone());
	let edit_video_edit_state = create_signal(ctx, entry.video_edit_state);
	let edit_poster_moment = create_signal(ctx, entry.poster_moment);
	let edit_notes_to_editor = create_signal(ctx, entry.notes_to_editor.clone());
	let edit_editor = create_signal(ctx, entry.editor.clone());
	let edit_is_incomplete = create_signal(ctx, entry.marked_incomplete);
	let edit_sort_key = create_signal(ctx, entry.manual_sort_key);

	let modified_data: &Signal<HashSet<ModifiedEventLogEntryParts>> = create_signal(ctx, HashSet::new());

	create_effect(ctx, || {
		if *edit_open_signal.get() {
			if let Some(entry) = event_log_entry_signal.get_untracked().as_ref() {
				edit_start_time.set(entry.start_time);
				edit_end_time.set(entry.end_time);
				edit_entry_type.set(entry.entry_type.clone());
				edit_description.set(entry.description.clone());
				edit_media_link.set(entry.media_link.clone());
				edit_submitter_or_winner.set(entry.submitter_or_winner.clone());
				edit_tags.set(entry.tags.clone());
				edit_video_edit_state.set(entry.video_edit_state);
				edit_poster_moment.set(entry.poster_moment);
				edit_notes_to_editor.set(entry.notes_to_editor.clone());
				edit_editor.set(entry.editor.clone());
				edit_is_incomplete.set(entry.marked_incomplete);
				edit_sort_key.set(entry.manual_sort_key);
			}
			modified_data.modify().clear();
		}
	});

	create_effect(ctx, || {
		edit_start_time.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::StartTime);
	});
	create_effect(ctx, || {
		edit_end_time.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::EndTime);
	});
	create_effect(ctx, || {
		edit_entry_type.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::EntryType);
	});
	create_effect(ctx, || {
		edit_description.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::Description);
	});
	create_effect(ctx, || {
		edit_media_link.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::MediaLink);
	});
	create_effect(ctx, || {
		edit_submitter_or_winner.track();
		modified_data
			.modify()
			.insert(ModifiedEventLogEntryParts::SubmitterOrWinner);
	});
	create_effect(ctx, || {
		edit_tags.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::Tags);
	});
	create_effect(ctx, || {
		edit_video_edit_state.track();
		modified_data
			.modify()
			.insert(ModifiedEventLogEntryParts::VideoEditState);
	});
	create_effect(ctx, || {
		edit_poster_moment.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::PosterMoment);
	});
	create_effect(ctx, || {
		edit_notes_to_editor.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::NotesToEditor);
	});
	create_effect(ctx, || {
		edit_editor.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::Editor);
	});
	create_effect(ctx, || {
		edit_is_incomplete.track();
		modified_data
			.modify()
			.insert(ModifiedEventLogEntryParts::MarkedIncomplete);
	});
	create_effect(ctx, || {
		edit_sort_key.track();
		modified_data.modify().insert(ModifiedEventLogEntryParts::SortKey);
	});

	let typing_events_signal = props.event_typing_events_signal.clone();
	let typing_data = create_memo(ctx, move || {
		let mut typing_data: HashMap<String, UserTypingData> = HashMap::new();
		for typing_value in typing_events_signal.get().iter().filter(|typing_event| {
			typing_event.event_log_entry.as_ref().map(|entry| &entry.id)
				== (*event_log_entry_signal.get()).as_ref().map(|entry| &entry.id)
		}) {
			let user = typing_value.user.clone();
			let (_, user_typing_data) = typing_data.entry(user.id.clone()).or_insert((user, HashMap::new()));
			user_typing_data.insert(typing_value.target_field, typing_value.data.clone());
		}
		typing_data
	});

	let row_edit_parent: &Signal<Option<EventLogEntry>> = create_signal(ctx, None);

	let child_event_signal = props.event_signal.clone();
	let child_entry_types_signal = props.entry_types_signal.clone();
	let child_all_log_entries_signal = props.all_log_entries.clone();

	view! {
		ctx,
		EventLogEntryRow(
			entry=event_log_entry_signal,
			event=(*event).clone(),
			entry_type=entry_type,
			click_handler=click_handler,
			jump_highlight_row_id=props.jump_highlight_row_id,
			new_entry_parent=props.new_entry_parent,
			child_depth=props.child_depth
		)
		EventLogEntryTyping(typing_data=typing_data)
		(if *edit_open_signal.get() {
			let close_handler = {
				let entry = entry.clone();
				let event_signal = event_signal.clone();
				let log_entries = log_entries.clone();
				move |edit_count: u8| {
					props.jump_highlight_row_id.set(String::new());

					let entry = entry.clone();
					let event_signal = event_signal.clone();
					let log_entries = log_entries.clone();
					spawn_local_scoped(ctx, async move {
						edit_open_signal.set(false);

						if edit_count == 0 {
							return;
						}

						let mut log_entries = log_entries.modify();
						let log_entry = log_entries.iter_mut().find(|log_entry| log_entry.id == entry.id);
						let log_entry = match log_entry {
							Some(entry) => entry,
							None => return
						};

						let event = (*event_signal.get()).clone();

						let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
						let mut ws = ws_context.lock().await;

						let modified_data = modified_data.modify();
						for changed_datum in modified_data.iter() {
							let event_message = match changed_datum {
								ModifiedEventLogEntryParts::StartTime => EventSubscriptionUpdate::ChangeStartTime(log_entry.clone(), *edit_start_time.get()),
								ModifiedEventLogEntryParts::EndTime => EventSubscriptionUpdate::ChangeEndTime(log_entry.clone(), *edit_end_time.get()),
								ModifiedEventLogEntryParts::EntryType => EventSubscriptionUpdate::ChangeEntryType(log_entry.clone(), (*edit_entry_type.get()).clone()),
								ModifiedEventLogEntryParts::Description => EventSubscriptionUpdate::ChangeDescription(log_entry.clone(), (*edit_description.get()).clone()),
								ModifiedEventLogEntryParts::MediaLink => EventSubscriptionUpdate::ChangeMediaLink(log_entry.clone(), (*edit_media_link.get()).clone()),
								ModifiedEventLogEntryParts::SubmitterOrWinner => EventSubscriptionUpdate::ChangeSubmitterWinner(log_entry.clone(), (*edit_submitter_or_winner.get()).clone()),
								ModifiedEventLogEntryParts::Tags => EventSubscriptionUpdate::ChangeTags(log_entry.clone(), (*edit_tags.get()).clone()),
								ModifiedEventLogEntryParts::VideoEditState => EventSubscriptionUpdate::ChangeVideoEditState(log_entry.clone(), *edit_video_edit_state.get()),
								ModifiedEventLogEntryParts::PosterMoment => EventSubscriptionUpdate::ChangePosterMoment(log_entry.clone(), *edit_poster_moment.get()),
								ModifiedEventLogEntryParts::NotesToEditor => EventSubscriptionUpdate::ChangeNotesToEditor(log_entry.clone(), (*edit_notes_to_editor.get()).clone()),
								ModifiedEventLogEntryParts::Editor => EventSubscriptionUpdate::ChangeEditor(log_entry.clone(), (*edit_editor.get()).clone()),
								ModifiedEventLogEntryParts::MarkedIncomplete => EventSubscriptionUpdate::ChangeIsIncomplete(log_entry.clone(), *edit_is_incomplete.get()),
								ModifiedEventLogEntryParts::SortKey => EventSubscriptionUpdate::ChangeManualSortKey(log_entry.clone(), *edit_sort_key.get())
							};
							let event_message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::EventUpdate(event.clone(), Box::new(event_message))));
							let event_message = match serde_json::to_string(&event_message) {
								Ok(msg) => msg,
								Err(error) => {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to serialize entry log change.", error));
									return;
								}
							};
							if let Err(error) = ws.send(Message::Text(event_message)).await {
								let data: &DataSignals = use_context(ctx);
								data.errors.modify().push(ErrorData::new_with_error("Failed to send log entry change.", error));
							}
						}
					});
				}
			};
			view! {
				ctx,
				EventLogEntryEdit(
					event=read_event_signal,
					permission_level=props.permission_level,
					event_entry_types=read_entry_types_signal,
					event_tags_name_index=tags_by_name_index,
					entry_types_datalist_id="event_entry_types",
					event_log_entry=event_log_entry_signal,
					tags_datalist_id="event_tags",
					start_time=edit_start_time,
					end_time=edit_end_time,
					entry_type=edit_entry_type,
					description=edit_description,
					media_link=edit_media_link,
					submitter_or_winner=edit_submitter_or_winner,
					tags=edit_tags,
					video_edit_state=edit_video_edit_state,
					poster_moment=edit_poster_moment,
					notes_to_editor=edit_notes_to_editor,
					editor=edit_editor,
					editor_name_index=editors_by_name_index,
					editor_name_datalist_id="editor_names",
					marked_incomplete=edit_is_incomplete,
					parent_log_entry=row_edit_parent,
					sort_key=edit_sort_key,
					close_handler=close_handler
				)
			}
		} else {
			view! { ctx, }
		})
		div(class="event_log_entry_children") {
			Keyed(
				iterable=child_log_entries,
				key=|entry| entry.id.clone(),
				view={
					let event_signal = child_event_signal.clone();
					let entry_types_signal = child_entry_types_signal.clone();
					let all_log_entries = child_all_log_entries_signal.clone();
					let typing_events = props.event_typing_events_signal.clone();
					move |ctx, entry| {
						let event_signal = event_signal.clone();
						let entry_types_signal = entry_types_signal.clone();
						let all_log_entries = all_log_entries.clone();
						let typing_events = typing_events.clone();
						view! {
							ctx,
							EventLogEntry(
								entry=entry,
								jump_highlight_row_id=props.jump_highlight_row_id,
								event_signal=event_signal,
								permission_level=props.permission_level,
								entry_types_signal=entry_types_signal,
								all_log_entries=all_log_entries,
								event_typing_events_signal=typing_events,
								can_edit=can_edit,
								tags_by_name_index=props.tags_by_name_index,
								editors_by_name_index=props.editors_by_name_index,
								read_event_signal=props.read_event_signal,
								read_entry_types_signal=props.read_entry_types_signal,
								new_entry_parent=props.new_entry_parent,
								entries_by_parent=props.entries_by_parent,
								child_depth=props.child_depth + 1
							)
						}
					}
				}
			)
		}
	}
}
