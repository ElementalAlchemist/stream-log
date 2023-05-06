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
}
