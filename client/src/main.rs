use gloo_net::websocket::futures::WebSocket;
use mogwai::prelude::*;
use std::panic;
use stream_log_shared::messages::initial::{InitialMessage, UserDataLoad};
use stream_log_shared::SYNC_VERSION;
use websocket::{read_websocket, WebSocketReadError};

mod dom;

mod error;
use error::{render_error_message, render_error_message_with_details};

mod pages;
use pages::{event_selection, register};

mod user_info_bar;

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
		let msg_data: InitialMessage = match read_websocket(&mut ws_read).await {
			Ok(msg) => msg,
			Err(error) => {
				match error {
					WebSocketReadError::ConnectionClosed => render_error_message("Unable to load/operate: Failed to receive initial websocket message (connection closed without content)"),
					WebSocketReadError::BinaryMessage => render_error_message("Unable to load/operate: Data received in an incorrect format"),
					WebSocketReadError::JsonError(error) => render_error_message_with_details("Unable to load/operate: Failed to deserialize initial message", error),
					WebSocketReadError::WebSocketError(error) => render_error_message_with_details("Unable to load/operate: Failed to receive initial websocket message", error)
				}
				return;
			}
		};
		if msg_data.sync_version != SYNC_VERSION {
			render_error_message("There was a version mismatch between the client and the server.");
			return;
		}
		let msg_user_data = msg_data.user_data;
		let user_data = match msg_user_data {
			UserDataLoad::User(user_data) => user_data,
			UserDataLoad::NewUser => match register::run_page(&mut ws_write, &mut ws_read).await {
				Ok(user) => user,
				Err(error) => {
					render_error_message_with_details("Failed to complete user registration", &error);
					return;
				}
			},
			UserDataLoad::MissingId => {
				render_error_message("User ID missing. Please reinitiate login workflow.");
				return;
			}
			UserDataLoad::Error => {
				render_error_message(
					"An error occurred reading user data. Please alert an administrator to this issue.",
				);
				return;
			}
		};

		let selected_event = match event_selection::run_page(&mut ws_write, &mut ws_read, &user_data).await {
			Ok(event) => event,
			Err(error) => {
				render_error_message_with_details("Failed to complete event selection", &error);
				return;
			}
		};
	});
}
