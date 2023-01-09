use crate::color_utils::{color_from_rgb_str, rgb_str_from_color};
use crate::components::color_input_with_contrast::ColorInputWithContrast;
use crate::pages::error::{ErrorData, ErrorView};
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashMap;
use stream_log_shared::messages::admin::AdminAction;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::{Event as WebEvent, HtmlButtonElement, HtmlInputElement};

async fn get_user_list(ctx: Scope<'_>) -> Result<Vec<UserData>, ()> {
	let ws_context: &Mutex<WebSocket> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let message = RequestMessage::Admin(AdminAction::ListUsers);
	let message_json = match serde_json::to_string(&message) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to serialize user list request message"),
				error,
			)));
			return Err(());
		}
	};
	if let Err(error) = ws.send(Message::Text(message_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error(
			String::from("Failed to send user list request message"),
			error,
		)));
		return Err(());
	}

	let user_list_response = read_websocket(&mut ws).await;
	let user_list: DataMessage<Vec<UserData>> = match user_list_response {
		Ok(resp) => resp,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to receive user list response message"),
				error,
			)));
			return Err(());
		}
	};

	match user_list {
		Ok(users) => Ok(users),
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("A server error occurred retrieving the user list"),
				error,
			)));
			Err(())
		}
	}
}

#[component]
async fn AdminManageUsersLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let Ok(user_list) = get_user_list(ctx).await else {
		return view! { ctx, ErrorView };
	};

	let user_list = create_signal(ctx, user_list);

	let changed_users: HashMap<String, UserData> = HashMap::new();
	let changed_users = create_signal(ctx, changed_users);
	let submit_button = create_node_ref(ctx);
	let cancel_button = create_node_ref(ctx);

	let form_submission_handler = move |event: WebEvent| {
		event.prevent_default();

		let submit_button_node: DomNode = submit_button.get();
		let submit_button: HtmlButtonElement = submit_button_node.unchecked_into();
		let cancel_button_node: DomNode = cancel_button.get();
		let cancel_button: HtmlButtonElement = cancel_button_node.unchecked_into();

		submit_button.set_disabled(true);
		cancel_button.set_disabled(true);

		spawn_local_scoped(ctx, async move {
			let changes: Vec<UserData> = changed_users.get().values().cloned().collect();

			let message = RequestMessage::Admin(AdminAction::EditUsers(changes));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						String::from("Failed to serialize user data"),
						error,
					)));
					navigate("/error");
					return;
				}
			};

			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("Failed to send user data updates"),
					error,
				)));
				navigate("/error");
				return;
			}
			navigate("/");
		});
	};

	let cancel_button_handler = |_event: WebEvent| {
		let submit_button_node: DomNode = submit_button.get();
		let submit_button: HtmlButtonElement = submit_button_node.unchecked_into();
		let cancel_button_node: DomNode = cancel_button.get();
		let cancel_button: HtmlButtonElement = cancel_button_node.unchecked_into();

		submit_button.set_disabled(true);
		cancel_button.set_disabled(true);

		navigate("/");
	};

	view! {
		ctx,
		h1 { "Manage Users" }
		form(on:submit=form_submission_handler) {
			table(id="admin_user_manage") {
				tr {
					th { "Username" }
					th { "Admin?" }
					th { "Color" }
				}
				Keyed(
					iterable=user_list,
					key=|user| user.id.clone(),
					view={
						move |ctx, user| {
							let checkbox = create_node_ref(ctx);
							let admin_change_handler = {
								let user = user.clone();
								move |_event: WebEvent| {
									let checkbox_ref: DomNode = checkbox.get();
									let checkbox: HtmlInputElement = checkbox_ref.unchecked_into();
									changed_users.modify().entry(user.id.clone()).or_insert_with(|| user.clone()).is_admin = checkbox.checked();
								}
							};

							let username_signal = create_signal(ctx, user.username.clone());
							let start_color = rgb_str_from_color(user.color);
							let color_signal = create_signal(ctx, start_color);

							create_effect(ctx, {
								let user = user.clone();
								move || {
									let new_color = (*color_signal.get()).clone();
									let Ok(new_color) = color_from_rgb_str(&new_color) else {
										return;
									};
									changed_users.modify().entry(user.id.clone()).or_insert_with(|| user.clone()).color = new_color;
								}
							});
							let color_view_id = format!("admin_user_color_{}", user.id);

							view! {
								ctx,
								tr {
									td { (user.username) }
									td(class="admin_user_admin_toggle") {
										input(type="checkbox", checked=user.is_admin, on:change=admin_change_handler, ref=checkbox)
									}
									td(class="admin_user_color_selection") {
										ColorInputWithContrast(color=color_signal, username=username_signal, view_id=&color_view_id)
									}
								}
							}
						}
					}
				)
			}
			button(ref=submit_button) { "Update" }
			button(type="button", on:click=cancel_button_handler, ref=cancel_button) { "Cancel" }
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
