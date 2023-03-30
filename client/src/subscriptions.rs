use crate::websocket::read_websocket;
use sycamore::prelude::*;

/// A struct containing all of the signals that can be updated by server messages.
pub struct DataSignals {}

impl<'a> DataSignals {
	pub fn new(ctx: Scope<'a>) -> Self {
		Self {}
	}
}

/// The message update loop
pub async fn process_messages() {}
