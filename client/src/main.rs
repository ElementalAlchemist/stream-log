use gloo_net::websocket::futures::WebSocket;
use stream_log_shared::messages::initial::{InitialMessage, UserDataLoad};
use stream_log_shared::SYNC_VERSION;
use sycamore::futures::spawn_local;
use websocket::websocket_endpoint;

mod error;
mod pages;
mod user_info_bar;
mod websocket;
use error::PageError;
use pages::error::error_message_view;
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
				let no_error: Option<PageError> = None;
				error_message_view(ctx, String::from("A mismatch in communication protocols occurred between the client and the server. Please refresh the page. If the problem persists, please contact an administrator."), no_error)
			});
			return;
		}

		match initial_message.user_data {
			UserDataLoad::User(user_data) => todo!(),
			UserDataLoad::NewUser => todo!(),
			UserDataLoad::MissingId => {
				sycamore::render(|ctx| {
					let no_error: Option<PageError> = None;
					error_message_view(
						ctx,
						String::from("An error occurred reading user data. Please log in again."),
						no_error,
					)
				});
			}
			UserDataLoad::Error => {
				sycamore::render(|ctx| {
					let no_error: Option<PageError> = None;
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
