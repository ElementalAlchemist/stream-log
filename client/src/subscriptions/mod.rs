use crate::websocket::read_websocket;
use futures::stream::SplitStream;
use gloo_net::websocket::futures::WebSocket;
use std::collections::HashMap;
use stream_log_shared::messages::subscriptions::InitialSubscriptionLoadData;
use stream_log_shared::messages::FromServerMessage;
use sycamore::prelude::*;

pub mod event;
use event::EventSubscriptionSignals;

pub mod registration;
use registration::RegistrationData;

/// A struct containing all of the signals that can be updated by server messages.
#[derive(Clone)]
pub struct DataSignals<'a> {
	pub errors: &'a Signal<Vec<String>>,
	pub events: HashMap<String, EventSubscriptionSignals>,
	pub registration: RegistrationData,
}

impl<'a> DataSignals<'a> {
	pub fn new(ctx: Scope<'_>) -> Self {
		Self {
			errors: create_signal(ctx, Vec::new()),
			events: HashMap::new(),
			registration: RegistrationData::new(),
		}
	}
}

/// The message update loop
pub async fn process_messages(ctx: Scope<'_>, mut ws_read: SplitStream<WebSocket>) {
	let data_signals: &RcSignal<DataSignals> = use_context(ctx);

	loop {
		let message: FromServerMessage = match read_websocket(&mut ws_read).await {
			Ok(msg) => msg,
			Err(_) => {
				data_signals.get().errors.modify().push(String::from(
					"The connection with the server has broken. If this wasn't expected, refresh the page.",
				));
				break;
			}
		};

		match message {
			FromServerMessage::InitialSubscriptionLoad(subscription_load_data) => match *subscription_load_data {
				InitialSubscriptionLoadData::Event(
					event,
					permission_level,
					entry_types,
					tags,
					editors,
					event_log_entries,
				) => {
					let permission = create_rc_signal(permission_level);
					let entry_types = create_rc_signal(entry_types);
					let tags = create_rc_signal(tags);
					let editors = create_rc_signal(editors);
					let event_log_entries = create_rc_signal(event_log_entries);

					let event_subscription_data = EventSubscriptionSignals {
						permission,
						entry_types,
						tags,
						editors,
						event_log_entries,
					};
					data_signals
						.modify()
						.events
						.insert(event.id.clone(), event_subscription_data);
				}
			},
			FromServerMessage::SubscriptionMessage(subscription_data) => { /* TODO */ }
			FromServerMessage::Unsubscribed(subscription_type) => { /* TODO */ }
			FromServerMessage::SubscriptionFailure(subscription_type, failure_info) => { /* TODO */ }
			FromServerMessage::RegistrationResponse(response) => { /* TODO */ }
		}
	}
}
