use chrono::{DateTime, Duration, Utc};
use gloo_timers::callback::Interval;
use std::rc::Rc;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::event_log::{EventLogEntry, EventLogTab};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::info_pages::InfoPage;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

pub struct EventSubscriptionSignalsInitData {
	pub event: Event,
	pub permission: PermissionLevel,
	pub entry_types: Vec<EntryType>,
	pub tags: Vec<Tag>,
	pub editors: Vec<UserData>,
	pub info_pages: Vec<InfoPage>,
	pub event_log_tabs: Vec<EventLogTab>,
	pub event_log_entries: Vec<EventLogEntry>,
}

#[derive(Clone)]
pub struct EventSubscriptionSignals {
	pub event: RcSignal<Event>,
	pub permission: RcSignal<PermissionLevel>,
	pub entry_types: RcSignal<Vec<EntryType>>,
	pub tags: RcSignal<Vec<Tag>>,
	pub editors: RcSignal<Vec<UserData>>,
	pub info_pages: RcSignal<Vec<InfoPage>>,
	pub event_log_tabs: RcSignal<Vec<EventLogTab>>,
	pub event_log_entries: RcSignal<Vec<EventLogEntry>>,
	pub typing_events: RcSignal<Vec<TypingEvent>>,
	_typing_expire_interval: Rc<Interval>,
}

impl EventSubscriptionSignals {
	pub fn new(init_data: EventSubscriptionSignalsInitData) -> Self {
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

		let event = create_rc_signal(init_data.event);
		let permission = create_rc_signal(init_data.permission);
		let entry_types = create_rc_signal(init_data.entry_types);
		let tags = create_rc_signal(init_data.tags);
		let editors = create_rc_signal(init_data.editors);
		let info_pages = create_rc_signal(init_data.info_pages);
		let event_log_tabs = create_rc_signal(init_data.event_log_tabs);
		let event_log_entries = create_rc_signal(init_data.event_log_entries);

		Self {
			event,
			permission,
			entry_types,
			tags,
			editors,
			info_pages,
			event_log_tabs,
			event_log_entries,
			typing_events,
			_typing_expire_interval,
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TypingTarget {
	Parent,
	StartTime,
	EndTime,
	EntryType,
	Description,
	MediaLink,
	SubmitterWinner,
	NotesToEditor,
}

#[derive(Clone, Debug)]
pub struct TypingEvent {
	pub event_log_entry: Option<EventLogEntry>,
	pub user: UserData,
	pub target_field: TypingTarget,
	pub data: String,
	pub time_received: DateTime<Utc>,
}
