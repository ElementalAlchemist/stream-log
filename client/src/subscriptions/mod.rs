use crate::websocket::read_websocket;
use futures::stream::SplitStream;
use gloo_net::websocket::futures::WebSocket;
use std::collections::HashMap;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::InitialSubscriptionLoadData;
use stream_log_shared::messages::tags::{Tag, TagEventAssociation};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::user_register::RegistrationResponse;
use stream_log_shared::messages::FromServerMessage;
use sycamore::prelude::*;

pub mod errors;
use errors::ErrorData;

pub mod event;
use event::EventSubscriptionSignals;

pub mod registration;
use registration::RegistrationData;

/// A struct containing all of the signals that can be updated by server messages.
#[derive(Clone)]
pub struct DataSignals<'a> {
	pub errors: &'a Signal<Vec<ErrorData>>,
	pub events: &'a Signal<HashMap<String, EventSubscriptionSignals>>,
	pub registration: RegistrationData<'a>,
	pub available_events: &'a Signal<Vec<Event>>,
	pub all_users: &'a Signal<Vec<UserData>>,
	pub all_events: &'a Signal<Vec<Event>>,
	pub all_tags: &'a Signal<Vec<Tag>>,
	pub tag_event_associations: &'a Signal<Vec<TagEventAssociation>>,
}

impl<'a> DataSignals<'a> {
	pub fn new(ctx: Scope<'_>) -> Self {
		Self {
			errors: create_signal(ctx, Vec::new()),
			events: create_signal(ctx, HashMap::new()),
			registration: RegistrationData::new(ctx),
			available_events: create_signal(ctx, Vec::new()),
			all_users: create_signal(ctx, Vec::new()),
			all_events: create_signal(ctx, Vec::new()),
			all_tags: create_signal(ctx, Vec::new()),
			tag_event_associations: create_signal(ctx, Vec::new()),
		}
	}
}

/// The message update loop
pub async fn process_messages(ctx: Scope<'_>, mut ws_read: SplitStream<WebSocket>) {
	let data_signals: &DataSignals = use_context(ctx);

	loop {
		let message: FromServerMessage = match read_websocket(&mut ws_read).await {
			Ok(msg) => msg,
			Err(_) => {
				data_signals.errors.modify().push(ErrorData::new(
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
					let event_id = event.id.clone();
					let event = create_rc_signal(event);
					let permission = create_rc_signal(permission_level);
					let entry_types = create_rc_signal(entry_types);
					let tags = create_rc_signal(tags);
					let editors = create_rc_signal(editors);
					let event_log_entries = create_rc_signal(event_log_entries);

					let event_subscription_data = EventSubscriptionSignals {
						event,
						permission,
						entry_types,
						tags,
						editors,
						event_log_entries,
					};
					data_signals.events.modify().insert(event_id, event_subscription_data);
				}
			},
			FromServerMessage::SubscriptionMessage(subscription_data) => { /* TODO */ }
			FromServerMessage::Unsubscribed(subscription_type) => { /* TODO */ }
			FromServerMessage::SubscriptionFailure(subscription_type, failure_info) => { /* TODO */ }
			FromServerMessage::RegistrationResponse(response) => match response {
				RegistrationResponse::UsernameCheck(check_data) => {
					data_signals.registration.username_check.set(Some(check_data))
				}
				RegistrationResponse::Finalize(registration_data) => {
					data_signals.registration.final_register.set(Some(registration_data))
				}
			},
		}
	}
}
