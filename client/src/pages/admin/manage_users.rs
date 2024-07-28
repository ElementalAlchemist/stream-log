// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::color_utils::{color_from_rgb_str, rgb_str_from_color};
use crate::components::color_input_with_contrast::ColorInputWithContrast;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use crate::websocket::WebSocketSendStream;
use futures::lock::Mutex;
use gloo_net::websocket::Message;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AdminManageUsersLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscription_result = {
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager
			.set_subscription(SubscriptionType::AdminUsers, &mut ws)
			.await
	};
	if let Err(error) = add_subscription_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send user list subscription message.",
			error,
		));
	}

	let all_users = create_memo(ctx, || (*data.all_users.get()).clone());

	view! {
		ctx,
		h1 { "Manage Users" }
		div(id="admin_user_manage") {
			div(id="admin_user_manage_headers", class="admin_user_manage_row") {
				div { "Username" }
				div { "Admin?" }
				div { "Color" }
				div { }
			}
			Keyed(
				iterable=all_users,
				key=|user| user.id.clone(),
				view={
					move |ctx, user| {
						let username_signal = create_signal(ctx, user.username.clone());
						let is_admin_signal = create_signal(ctx, user.is_admin);
						let start_color = rgb_str_from_color(user.color);
						let color_signal = create_signal(ctx, start_color);

						let color_view_id = format!("admin_user_color_{}", user.id);

						let form_submit_handler = {
							let user = user.clone();
							move |_event: WebEvent| {
								let Ok(new_color) = color_from_rgb_str(&color_signal.get()) else {
									return;
								};
								let updated_user = UserData {
									id: user.id.clone(),
									username: (*username_signal.get()).clone(),
									color: new_color,
									is_admin: *is_admin_signal.get()
								};

								spawn_local_scoped(ctx, async move {
									let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
									let mut ws = ws_context.lock().await;

									let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminUserUpdate(updated_user)));
									let message_json = match serde_json::to_string(&message) {
										Ok(msg) => msg,
										Err(error) => {
											let data: &DataSignals = use_context(ctx);
											data.errors.modify().push(ErrorData::new_with_error("Failed to serialize user update message.", error));
											return;
										}
									};
									if let Err(error) = ws.send(Message::Text(message_json)).await {
										let data: &DataSignals = use_context(ctx);
										data.errors.modify().push(ErrorData::new_with_error("Failed to send user update message.", error));
									}
								});
							}
						};

						view! {
							ctx,
							form(class="admin_user_manage_row", on:submit=form_submit_handler) {
								div { (user.username) }
								div(class="admin_user_admin_toggle") {
									input(type="checkbox", bind:checked=is_admin_signal)
								}
								div(class="admin_user_color_selection") {
									ColorInputWithContrast(color=color_signal, username=username_signal, view_id=&color_view_id)
								}
								div(class="admin_user_manage_submit") {
									button { "Update" }
								}
							}
						}
					}
				}
			)
		}
	}
}

#[component]
pub fn AdminManageUsersView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let user_signal: &Signal<Option<UserData>> = use_context(ctx);
	match user_signal.get().as_ref() {
		Some(user) => {
			if !user.is_admin {
				spawn_local_scoped(ctx, async {
					navigate("/");
				});
				return view! { ctx, };
			}
		}
		None => {
			spawn_local_scoped(ctx, async {
				navigate("/register");
			});
			return view! { ctx, };
		}
	}

	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading users..." }) {
			AdminManageUsersLoadedView
		}
	}
}
