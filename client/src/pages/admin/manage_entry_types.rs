use crate::color_utils::{color_from_rgb_str, rgb_str_from_color};
use crate::event_type_colors::{use_white_foreground, BLACK, WHITE};
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashSet;
use stream_log_shared::messages::admin::AdminEntryTypeUpdate;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

const DEFAULT_COLOR: &str = "#ffffff";

#[component]
async fn AdminManageEntryTypesLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscription_result = {
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager
			.set_subscription(SubscriptionType::AdminEntryTypes, &mut ws)
			.await
	};
	if let Err(error) = add_subscription_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send entry type subscription message.",
			error,
		));
	}

	let all_entry_types = create_memo(ctx, || (*data.all_entry_types.get()).clone());

	let used_names_signal = create_memo(ctx, || {
		let names: HashSet<String> = data
			.all_entry_types
			.get()
			.iter()
			.map(|entry_type| entry_type.name.clone())
			.collect();
		names
	});

	let new_type_name_signal = create_signal(ctx, String::new());
	let new_type_name_error_signal = create_signal(ctx, String::new());
	let new_type_color_signal = create_signal(ctx, String::from(DEFAULT_COLOR));
	let new_type_color_error_signal = create_signal(ctx, String::new());
	let new_type_display_style_signal = create_memo(ctx, || {
		let background = new_type_color_signal.get();
		let foreground = match color_from_rgb_str(&background) {
			Ok(color) => {
				if use_white_foreground(&color) {
					WHITE
				} else {
					BLACK
				}
			}
			Err(_) => BLACK,
		};
		let foreground = rgb_str_from_color(foreground);
		format!("font-weight: 700, background: {}, color: {}", background, foreground)
	});

	let new_type_submit_handler = move |event: WebEvent| {
		event.prevent_default();

		let name = (*new_type_name_signal.get()).clone();
		if name.is_empty() {
			new_type_name_error_signal.set(String::from("Name must not be empty."));
			return;
		}
		if used_names_signal.get().contains(&name) {
			new_type_name_error_signal.set(String::from("That name is already in use."));
			return;
		}
		new_type_name_error_signal.modify().clear();

		let color = match color_from_rgb_str(&new_type_color_signal.get()) {
			Ok(color) => color,
			Err(error) => {
				new_type_color_error_signal.set(format!("Invalid color: {}", error));
				return;
			}
		};
		new_type_color_error_signal.modify().clear();

		new_type_name_signal.modify().clear();
		new_type_color_signal.set(String::from(DEFAULT_COLOR));

		let new_type = EntryType {
			id: String::new(),
			name,
			color,
		};
		let message = FromClientMessage::SubscriptionMessage(Box::new(
			SubscriptionTargetUpdate::AdminEntryTypesUpdate(AdminEntryTypeUpdate::UpdateEntryType(new_type)),
		));
		let message_json = match serde_json::to_string(&message) {
			Ok(msg) => msg,
			Err(error) => {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to serialize new entry type message.",
					error,
				));
				return;
			}
		};

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to send new entry type message.",
					error,
				));
			}
		});
	};

	let done_click_handler = move |_event: WebEvent| {
		navigate("/");
	};

	view! {
		ctx,
		div(id="admin_manage_entry_types") {
			Keyed(
				iterable=all_entry_types,
				key=|entry_type| entry_type.id.clone(),
				view=move |ctx, entry_type| {
					let name_signal = create_signal(ctx, entry_type.name.clone());
					let name_error_signal = create_signal(ctx, String::new());
					let color_signal = create_signal(ctx, rgb_str_from_color(entry_type.color));
					let color_error_signal = create_signal(ctx, String::new());

					let display_style_signal = create_memo(ctx, || {
						let background = color_signal.get();
						let foreground = match color_from_rgb_str(&background) {
							Ok(color) => {
								if use_white_foreground(&color) {
									WHITE
								} else {
									BLACK
								}
							}
							Err(_) => BLACK
						};
						let foreground = rgb_str_from_color(foreground);
						format!("font-weight: 700; background: {}, color: {}", background, foreground)
					});

					let update_type_handler = move |event: WebEvent| {
						event.prevent_default();

						let name = (*name_signal.get()).clone();
						if name.is_empty() {
							name_error_signal.set(String::from("Name must not be empty"));
							return;
						}
						if used_names_signal.get().contains(&name) {
							name_error_signal.set(String::from("Name is already in use"));
							return;
						}
						name_error_signal.modify().clear();

						let color = color_signal.get();
						let color = match color_from_rgb_str(&color) {
							Ok(color) => color,
							Err(error) => {
								color_error_signal.set(format!("Invalid color: {}", error));
								return;
							}
						};
						color_error_signal.modify().clear();

						let updated_type = EntryType { id: entry_type.id.clone(), name, color };
						let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEntryTypesUpdate(AdminEntryTypeUpdate::UpdateEntryType(updated_type))));
						let message_json = match serde_json::to_string(&message) {
							Ok(msg) => msg,
							Err(error) => {
								let data: &DataSignals = use_context(ctx);
								data.errors.modify().push(ErrorData::new_with_error("Failed to serialize entry type update message.", error));
								return;
							}
						};

						spawn_local_scoped(ctx, async move {
							let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
							let mut ws = ws_context.lock().await;

							if let Err(error) = ws.send(Message::Text(message_json)).await {
								let data: &DataSignals = use_context(ctx);
								data.errors.modify().push(ErrorData::new_with_error("Failed to send entry type update message.", error));
							}
						});
					};

					view! {
						ctx,
						form(class="admin_manage_entry_types_row", on:submit=update_type_handler) {
							div(style=display_style_signal.get()) {
								(name_signal.get())
							}
							div {
								input(bind:value=name_signal, class=if name_error_signal.get().is_empty() { "" } else { "error" }, title=*name_error_signal.get())
							}
							div {
								input(type="color", bind:value=color_signal, class=if color_error_signal.get().is_empty() { "" } else { "error" }, title=*color_error_signal.get())
							}
							div {
								button(type="submit") { "Update" }
							}
						}
					}
				}
			)
			form(class="admin_manage_entry_types_row", on:submit=new_type_submit_handler) {
				div(style=new_type_display_style_signal.get()) {
					(new_type_name_signal.get())
				}
				div {
					input(bind:value=new_type_name_signal, class=if new_type_name_error_signal.get().is_empty() { "" } else { "error" }, title=*new_type_name_error_signal.get())
				}
				div {
					input(type="color", bind:value=new_type_color_signal, class=if new_type_color_error_signal.get().is_empty() { "" } else { "error" }, title=*new_type_color_error_signal.get())
				}
				div {
					button(type="submit") { "Add New" }
				}
			}
		}
		button(on:click=done_click_handler) { "Done" }
	}
}

#[component]
pub fn AdminManageEntryTypesView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let user_signal: &Signal<Option<UserData>> = use_context(ctx);

	if let Some(user_data) = user_signal.get().as_ref() {
		if !user_data.is_admin {
			spawn_local_scoped(ctx, async {
				navigate("/");
			});
			return view! { ctx, };
		}
	} else {
		spawn_local_scoped(ctx, async {
			navigate("/");
		});
		return view! { ctx, };
	}

	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading event types data..." }) {
			AdminManageEntryTypesLoadedView
		}
	}
}
