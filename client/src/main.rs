use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use mogwai::prelude::*;
use std::panic;
use stream_log_shared::messages::initial::{InitialMessage, UserDataLoad};
use stream_log_shared::SYNC_VERSION;

mod error;
use error::{render_error_message, render_error_message_with_details};

mod pages;
use pages::register;

mod websocket;
use websocket::websocket_endpoint;

fn main() {
	panic::set_hook(Box::new(console_error_panic_hook::hook));

	mogwai::spawn(async {
		let ws = match WebSocket::open(websocket_endpoint().as_str()) {
			Ok(ws) => ws,
			Err(error) => {
				render_error_message_with_details(
					"Unable to load/operate: A websocket connection could not be formed.",
					error,
				);
				return;
			}
		};
		let (mut ws_write, mut ws_read) = ws.split();
		let msg = match ws_read.next().await {
			Some(Ok(msg)) => msg,
			Some(Err(error)) => {
				render_error_message_with_details(
					"Unable to load/operate: Failed to receive initial websocket message.",
					error,
				);
				return;
			}
			None => {
				render_error_message("Unable to load/operate: Failed to receive initial websocket message (connection closed without content)");
				return;
			}
		};
		let msg = match msg {
			Message::Text(txt) => txt,
			Message::Bytes(_) => unimplemented!(),
		};
		let msg_data: InitialMessage = serde_json::from_str(&msg).expect("Message data was of the incorrect type");
		if msg_data.sync_version != SYNC_VERSION {
			render_error_message("There was a version mismatch between the client and the server.");
			return;
		}
		let msg_user_data = msg_data.user_data;
		match msg_user_data {
			UserDataLoad::User(user_data) => todo!(),
			UserDataLoad::NewUser => {
				if let Err(error_msg) = register::run_page(&mut ws_write, &mut ws_read).await {
					render_error_message_with_details("Failed to complete user registration", &error_msg);
					return;
				}
			}
			UserDataLoad::MissingId => render_error_message("User ID missing. Please reinitiate login workflow."),
			UserDataLoad::Error => render_error_message(
				"An error occurred reading user data. Please alert an administrator to this issue.",
			),
		}
	});
}
