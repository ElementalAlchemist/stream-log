use gloo_net::websocket::futures::WebSocket;
use std::collections::HashSet;
use stream_log_shared::messages::initial::{InitialMessage, UserDataLoad};
use stream_log_shared::SYNC_VERSION;
use sycamore::futures::spawn_local;
use sycamore::prelude::*;
use websocket::websocket_endpoint;

mod pages;
mod websocket;
use pages::error::error_message_view;
use pages::event_selection::handle_event_selection_page;
use pages::register::handle_registration_page;
use websocket::read_websocket;

fn main() {
	console_error_panic_hook::set_once();

	spawn_local(async {
		let ws = WebSocket::open(websocket_endpoint().as_str());
		let mut ws = match ws {
			Ok(ws) => ws,
			Err(error) => {
				sycamore::render(|ctx| {
					error_message_view(
						ctx,
						String::from("Unable to load/operate: Failed to form a websocket connection"),
						Some(error),
					)
				});
				return;
			}
		};

		let initial_message: InitialMessage = match read_websocket(&mut ws).await {
			Ok(msg) => msg,
			Err(error) => {
				sycamore::render(|ctx| {
					error_message_view(
						ctx,
						String::from("Unable to load/operate: Failed to read initial info message"),
						Some(error),
					)
				});
				return;
			}
		};

		if initial_message.sync_version != SYNC_VERSION {
			sycamore::render(|ctx| {
				let no_error: Option<String> = None;
				error_message_view(ctx, String::from("A mismatch in communication protocols occurred between the client and the server. Please refresh the page. If the problem persists, please contact an administrator."), no_error)
			});
			return;
		}

		match initial_message.user_data {
			UserDataLoad::User(user_data) => {
				let user_signal = create_rc_signal(Some(user_data));
				let suppressible_user_bar_parts = create_rc_signal(HashSet::new());
				handle_event_selection_page(user_signal, &mut ws, suppressible_user_bar_parts).await;
			}
			UserDataLoad::NewUser => handle_registration_page(ws).await,
			UserDataLoad::MissingId => {
				sycamore::render(|ctx| {
					let no_error: Option<String> = None;
					error_message_view(
						ctx,
						String::from("An error occurred reading user data. Please log in again."),
						no_error,
					)
				});
			}
			UserDataLoad::Error => {
				sycamore::render(|ctx| {
					let no_error: Option<String> = None;
					error_message_view(
						ctx,
						String::from(
							"An error occurred with logging in. Please contact an administrator regarding this issue.",
						),
						no_error,
					)
				});
			}
		}
	});
}
