use crate::websocket::read_websocket;
use futures::stream::SplitStream;
use gloo_net::websocket::futures::WebSocket;
use stream_log_shared::messages::FromServerMessage;
use sycamore::prelude::*;

/// A struct containing all of the signals that can be updated by server messages.
#[derive(Clone)]
pub struct DataSignals<'a> {
	pub errors: &'a Signal<Vec<String>>,
}

impl<'a> DataSignals<'a> {
	pub fn new(ctx: Scope<'_>) -> Self {
		Self {
			errors: create_signal(ctx, Vec::new()),
		}
	}
}

/// The message update loop
pub async fn process_messages(ctx: Scope<'_>, mut ws_read: SplitStream<WebSocket>) {
	let data_signals: &Signal<DataSignals> = use_context(ctx);

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
			FromServerMessage::InitialSubscriptionLoad(subscription_load_data) => { /* TODO */ }
			FromServerMessage::SubscriptionMessage(subscription_data) => { /* TODO */ }
			FromServerMessage::Unsubscribed(subscription_type) => { /* TODO */ }
			FromServerMessage::SubscriptionFailure(subscription_type, failure_info) => { /* TODO */ }
			FromServerMessage::RegistrationResponse(response) => { /* TODO */ }
		}
	}
}
