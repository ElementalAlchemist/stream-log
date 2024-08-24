// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::color_utils::{color_from_rgb_str, rgb_str_from_color};
use crate::components::color_input_with_contrast::ColorInputWithContrast;
use crate::page_utils::set_page_title;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::DataSignals;
use crate::websocket::WebSocketSendStream;
use futures::lock::Mutex;
use gloo_net::websocket::Message;
use stream_log_shared::messages::user::{SelfUserData, UpdateUser};
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
pub fn UserProfileView<G: Html>(ctx: Scope<'_>) -> View<G> {
	set_page_title("Profile | Stream Log");

	let user_signal: &Signal<Option<SelfUserData>> = use_context(ctx);
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
					let data: &RcSignal<DataSignals> = use_context(ctx);
					data.get()
						.errors
						.modify()
						.push(ErrorData::new_with_error("Failed to handle new color", error));
					return;
				}
			};

			let message = FromClientMessage::UpdateProfile(UpdateUser { color: new_color });
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &RcSignal<DataSignals> = use_context(ctx);
					data.get().errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize user color update request",
						error,
					));
					return;
				}
			};

			spawn_local_scoped(ctx, {
				let user_data = user_data.clone();
				async move {
					let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
					let mut ws = ws_context.lock().await;

					if let Err(error) = ws.send(Message::Text(message_json)).await {
						let data: &RcSignal<DataSignals> = use_context(ctx);
						data.get().errors.modify().push(ErrorData::new_with_error(
							"Failed to send user color update request",
							error,
						));
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
