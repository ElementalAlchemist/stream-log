use chrono::{DateTime, Duration, Utc};
use gloo_timers::callback::Interval;
use std::rc::Rc;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::EventLogEntry;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

#[derive(Clone)]
pub struct EventSubscriptionSignals {
	pub event: RcSignal<Event>,
	pub permission: RcSignal<PermissionLevel>,
	pub entry_types: RcSignal<Vec<EntryType>>,
	pub editors: RcSignal<Vec<UserData>>,
	pub event_log_entries: RcSignal<Vec<EventLogEntry>>,
	pub typing_events: RcSignal<Vec<TypingEvent>>,
	_typing_expire_interval: Rc<Interval>,
}

impl EventSubscriptionSignals {
	pub fn new(
		event: Event,
		permission: PermissionLevel,
		entry_types: Vec<EntryType>,
		editors: Vec<UserData>,
		event_log_entries: Vec<EventLogEntry>,
	) -> Self {
		let typing_events: RcSignal<Vec<TypingEvent>> = create_rc_signal(Vec::new());
		let typing_expire_interval = Interval::new(10_000, {
			let typing_events = typing_events.clone();
			move || {
				let mut typing_events = typing_events.modify();
				let expire_time = Utc::now() - Duration::seconds(30);
				typing_events.retain(|event| event.time_received > expire_time);
			}
		});
		let _typing_expire_interval = Rc::new(typing_expire_interval);

		let event = create_rc_signal(event);
		let permission = create_rc_signal(permission);
		let entry_types = create_rc_signal(entry_types);
		let editors = create_rc_signal(editors);
		let event_log_entries = create_rc_signal(event_log_entries);

		Self {
			event,
			permission,
			entry_types,
			editors,
			event_log_entries,
			typing_events,
			_typing_expire_interval,
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TypingTarget {
	StartTime,
	EndTime,
	EntryType,
	Description,
	MediaLink,
	SubmitterWinner,
	NotesToEditor,
}

#[derive(Clone)]
pub struct TypingEvent {
	pub event_log_entry: Option<EventLogEntry>,
	pub user: UserData,
	pub target_field: TypingTarget,
	pub data: String,
	pub time_received: DateTime<Utc>,
}
