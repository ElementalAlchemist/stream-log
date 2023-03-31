use super::error::ErrorData;
use crate::color_utils::{color_from_rgb_str, rgb_str_from_color};
use crate::components::color_input_with_contrast::ColorInputWithContrast;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::user::{UpdateUser, UserData};
use stream_log_shared::messages::RequestMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
pub fn UserProfileView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let user_signal: &Signal<Option<UserData>> = use_context(ctx);
	let user_data = match (*user_signal.get()).clone() {
		Some(data) => data,
		None => {
			spawn_local_scoped(ctx, async {
				navigate("/");
			});
			return view! { ctx, };
		}
	};

	let default_color = rgb_str_from_color(user_data.color);

	let color_signal = create_signal(ctx, default_color);
	let username_signal = create_signal(ctx, user_data.username.clone());

	let submit_profile_handler = {
		let user_data = user_data.clone();
		move |event: WebEvent| {
			event.prevent_default();

			let new_color = match color_from_rgb_str(color_signal.get().as_str()) {
				Ok(color) => color,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error("Failed to handle new color", error)));
					navigate("/error");
					return;
				}
			};

			let message = RequestMessage::UpdateProfile(UpdateUser::UpdateColor(new_color));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize user color update request",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			spawn_local_scoped(ctx, {
				let user_data = user_data.clone();
				async move {
					let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
					let mut ws = ws_context.lock().await;

					if let Err(error) = ws.send(Message::Text(message_json)).await {
						let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
						error_signal.set(Some(ErrorData::new_with_error(
							"Failed to send user color update request",
							error,
						)));
						navigate("/error");
						return;
					}

					let mut new_user = user_data.clone();
					new_user.color = new_color;
					user_signal.set(Some(new_user));
					navigate("/");
				}
			});
		}
	};

	view! {
		ctx,
		h1 { (user_data.username) }
		form(id="user_profile_edit", on:submit=submit_profile_handler) {
			ColorInputWithContrast(color=color_signal, username=username_signal, view_id="user_profile")
			button(type="submit") { "Update" }
		}
	}
}
