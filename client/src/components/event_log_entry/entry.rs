// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::row::EventLogEntryRow;
use super::typing::EventLogEntryTyping;
use super::UserTypingData;
use crate::subscriptions::event::EventSubscriptionSignals;
use std::collections::HashMap;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::EventLogEntry;
use sycamore::prelude::*;

#[derive(Prop)]
pub struct EventLogEntryProps<'a> {
	entry: EventLogEntry,
	jump_highlight_row_id: &'a Signal<String>,
	event_subscription_data: EventSubscriptionSignals,
	can_edit: &'a ReadSignal<bool>,
	editing_log_entry: &'a Signal<Option<EventLogEntry>>,
	read_entry_types_signal: &'a ReadSignal<Vec<EntryType>>,
	editing_entry_parent: &'a Signal<Option<EventLogEntry>>,
	entries_by_parent: &'a ReadSignal<HashMap<String, Vec<EventLogEntry>>>,
	child_depth: u32,
	entry_numbers: &'a ReadSignal<HashMap<String, usize>>,
	use_editor_view: &'a ReadSignal<bool>,
}

#[component]
pub fn EventLogEntry<'a, G: Html>(ctx: Scope<'a>, props: EventLogEntryProps<'a>) -> View<G> {
	let entry = props.entry;
	let can_edit = props.can_edit;

	let event_signal = props.event_subscription_data.event.clone();
	let entry_types_signal = props.event_subscription_data.entry_types.clone();
	let log_entries = props.event_subscription_data.event_log_entries.clone();

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
			entry
				.entry_type
				.as_ref()
				.and_then(|entry_type| entry_types.iter().find(|et| et.id == *entry_type).cloned())
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

	let typing_events_signal = props.event_subscription_data.typing_events.clone();
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

	let row_event_subscription_data = props.event_subscription_data.clone();

	view! {
		ctx,
		EventLogEntryRow(
			entry=event_log_entry_signal,
			event_subscription_data=row_event_subscription_data,
			can_edit=can_edit,
			entry_type=entry_type,
			jump_highlight_row_id=props.jump_highlight_row_id,
			editing_log_entry=props.editing_log_entry,
			editing_entry_parent=props.editing_entry_parent,
			child_depth=props.child_depth,
			entry_numbers=props.entry_numbers,
			use_editor_view=props.use_editor_view
		)
		EventLogEntryTyping(
			event=event_signal,
			event_entry_types=props.read_entry_types_signal,
			event_log=log_entries,
			typing_data=typing_data,
			use_editor_view=props.use_editor_view
		)
		div(class="event_log_entry_children") {
			Keyed(
				iterable=child_log_entries,
				key=|entry| entry.id.clone(),
				view={
					let event_subscription_data = props.event_subscription_data.clone();
					move |ctx, entry| {
						let event_subscription_data = event_subscription_data.clone();
						view! {
							ctx,
							EventLogEntry(
								entry=entry,
								jump_highlight_row_id=props.jump_highlight_row_id,
								event_subscription_data=event_subscription_data,
								can_edit=can_edit,
								editing_log_entry=props.editing_log_entry,
								read_entry_types_signal=props.read_entry_types_signal,
								editing_entry_parent=props.editing_entry_parent,
								entries_by_parent=props.entries_by_parent,
								child_depth=props.child_depth + 1,
								entry_numbers=props.entry_numbers,
								use_editor_view=props.use_editor_view
							)
						}
					}
				}
			)
		}
	}
}
