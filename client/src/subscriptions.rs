use crate::websocket::read_websocket;
use futures::stream::SplitStream;
use gloo_net::websocket::futures::WebSocket;
use sycamore::prelude::*;

/// A struct containing all of the signals that can be updated by server messages.
pub struct DataSignals {}

impl<'a> DataSignals {
	pub fn new(ctx: Scope<'a>) -> Self {
		Self {}
	}
}

/// The message update loop
pub async fn process_messages(ctx: Scope<'_>, ws_read: SplitStream<WebSocket>) {
	let data_signals: &Signal<DataSignals> = use_context(ctx);

	// TODO Read messages
}
