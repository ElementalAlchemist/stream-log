use futures::StreamExt;
use gloo_net::websocket::futures::WebSocket;
use stream_log_shared::messages::initial::{InitialMessage, UserDataLoad};
use stream_log_shared::SYNC_VERSION;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use websocket::websocket_endpoint;

mod app;
mod pages;
mod user_info_bar;
mod websocket;
use pages::error::error_message_view;
use user_info_bar::render_user_info_bar;
use websocket::read_websocket;

use app::App;

fn main() {
	console_error_panic_hook::set_once();

	sycamore::render(|ctx| {
		let user_bar_signal = create_signal(ctx, render_user_info_bar(ctx, None, &[]));
		let ws = WebSocket::open(websocket_endpoint().as_str());
		let ws = match ws {
			Ok(ws) => ws,
			Err(error) => {
				let view = error_message_view(
					ctx,
					String::from("Unable to load/operate: Failed to form a websocket connection"),
					Some(error),
				);
				let app_signal = create_signal(ctx, view);
				return view! { ctx, App { page: app_signal, user_bar: user_bar_signal }};
			}
		};

		let render_signal = create_signal(ctx, view! { ctx, });
		let (mut ws_write, mut ws_read) = ws.split();

		spawn_local_scoped(ctx, async move {
			let initial_message: InitialMessage = match read_websocket(&mut ws_read).await {
				Ok(msg) => msg,
				Err(error) => {
					let view = error_message_view(
						ctx,
						String::from("Unable to load/operate: Failed to read initial info message"),
						Some(error),
					);
					render_signal.set(view);
					return;
				}
			};

			if initial_message.sync_version != SYNC_VERSION {
				let no_error: Option<String> = None;
				let view = error_message_view(ctx, String::from("A mismatch in communication occurred between the client and the server. Please refresh the page."), no_error);
				render_signal.set(view);
				return;
			}

			match initial_message.user_data {
				UserDataLoad::User(user_data) => todo!(),
				UserDataLoad::NewUser => todo!(),
				UserDataLoad::MissingId => {
					let no_error: Option<String> = None;
					let view = error_message_view(
						ctx,
						String::from("An error occurred reading user data. Please log in again."),
						no_error,
					);
					render_signal.set(view);
				}
				UserDataLoad::Error => {
					let no_error: Option<String> = None;
					let view = error_message_view(
						ctx,
						String::from(
							"An error occurred with logging in. Please contact an administrator regarding this issue.",
						),
						no_error,
					);
					render_signal.set(view);
				}
			}
		});

		view! { ctx, App { page: render_signal, user_bar: user_bar_signal } }
	});
}
