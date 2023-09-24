use super::row::EventLogEntryRow;
use super::typing::EventLogEntryTyping;
use super::UserTypingData;
use crate::subscriptions::event::TypingEvent;
use std::collections::HashMap;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use sycamore::prelude::*;

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
	editing_log_entry: &'a Signal<Option<EventLogEntry>>,
	read_entry_types_signal: &'a ReadSignal<Vec<EntryType>>,
	editing_entry_parent: &'a Signal<Option<EventLogEntry>>,
	entries_by_parent: &'a ReadSignal<HashMap<String, Vec<EventLogEntry>>>,
	child_depth: u32,
}

#[component]
pub fn EventLogEntry<'a, G: Html>(ctx: Scope<'a>, props: EventLogEntryProps<'a>) -> View<G> {
	let entry = props.entry;
	let can_edit = props.can_edit;

	let event_signal = props.event_signal.clone();
	let entry_types_signal = props.entry_types_signal.clone();
	let log_entries = props.all_log_entries.clone();

	let event = event_signal.get();
	let click_handler = if *can_edit.get() {
		let entry = entry.clone();
		let log_entries = log_entries.clone();
		Some(move || {
			let Some(current_entry) = log_entries
				.get()
				.iter()
				.find(|log_entry| log_entry.id == entry.id)
				.cloned()
			else {
				return;
			};
			props.editing_log_entry.set(Some(current_entry));
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

	let child_event_signal = props.event_signal.clone();
	let child_entry_types_signal = props.entry_types_signal.clone();
	let child_all_log_entries_signal = props.all_log_entries.clone();
	let typing_event_signal = event_signal.clone();

	view! {
		ctx,
		EventLogEntryRow(
			entry=event_log_entry_signal,
			event=(*event).clone(),
			entry_type=entry_type,
			click_handler=click_handler,
			jump_highlight_row_id=props.jump_highlight_row_id,
			editing_entry_parent=props.editing_entry_parent,
			child_depth=props.child_depth
		)
		EventLogEntryTyping(event=typing_event_signal, event_entry_types=props.read_entry_types_signal, event_log=props.all_log_entries, typing_data=typing_data)
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
								editing_log_entry=props.editing_log_entry,
								read_entry_types_signal=props.read_entry_types_signal,
								editing_entry_parent=props.editing_entry_parent,
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
