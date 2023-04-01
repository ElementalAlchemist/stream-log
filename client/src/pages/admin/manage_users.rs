use crate::color_utils::{color_from_rgb_str, rgb_str_from_color};
use crate::components::color_input_with_contrast::ColorInputWithContrast;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashMap;
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
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let message = FromClientMessage::StartSubscription(SubscriptionType::AdminUsers);
	let message_json = match serde_json::to_string(&message) {
		Ok(msg) => msg,
		Err(error) => {
			data.errors.modify().push(ErrorData::new_with_error(
				"Failed to serialize user list subscription message.",
				error,
			));
			return view! { ctx, };
		}
	};
	if let Err(error) = ws.send(Message::Text(message_json)).await {
		data.errors.modify().push(ErrorData::new_with_error(
			"Failed to send user list subscription message.",
			error,
		));
	}

	let changed_users: HashMap<String, UserData> = HashMap::new();
	let changed_users = create_signal(ctx, changed_users);

	let done_button_handler = move |_event: WebEvent| {
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::EndSubscription(SubscriptionType::AdminUsers);
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize admin users unsubscribe message.",
						error,
					));
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to send admin users unsubscribe message.",
					error,
				));
			}
		});

		navigate("/");
	};

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
				iterable=data.all_users,
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
								let Ok(new_color) = color_from_rgb_str(&*color_signal.get()) else {
									return;
								};
								let updated_user = UserData {
									id: user.id.clone(),
									username: (*username_signal.get()).clone(),
									color: new_color,
									is_admin: *is_admin_signal.get()
								};

								spawn_local_scoped(ctx, async move {
									let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
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
		button(type="button", on:click=done_button_handler) { "Done" }
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
