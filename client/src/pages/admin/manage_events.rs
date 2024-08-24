// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::entry_utils::{parse_time_field_value, ISO_DATETIME_FORMAT_STRING};
use crate::page_utils::set_page_title;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use crate::websocket::WebSocketSendStream;
use chrono::prelude::*;
use futures::lock::Mutex;
use gloo_net::websocket::Message;
use std::collections::HashSet;
use stream_log_shared::messages::admin::AdminEventUpdate;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AdminManageEventsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	set_page_title("Manage Events | Stream Log");

	let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscription_result = {
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager
			.set_subscription(SubscriptionType::AdminEvents, &mut ws)
			.await
	};
	if let Err(error) = add_subscription_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send event subscription message.",
			error,
		));
	}

	let all_events = create_memo(ctx, || (*data.all_events.get()).clone());

	let used_names_signal = create_memo(ctx, || {
		let names: HashSet<String> = data.all_events.get().iter().map(|event| event.name.clone()).collect();
		names
	});

	let new_event_name_signal = create_signal(ctx, String::new());
	let new_event_name_error_signal = create_signal(ctx, String::new());
	let new_event_time_signal = create_signal(ctx, format!("{}", Utc::now().format(ISO_DATETIME_FORMAT_STRING)));
	let new_event_time_error_signal = create_signal(ctx, String::new());
	let new_event_editor_link_format_signal = create_signal(ctx, String::new());
	let new_event_first_tab_name_signal = create_signal(ctx, String::new());

	let new_event_submit_handler = move |event: WebEvent| {
		event.prevent_default();

		let name = (*new_event_name_signal.get()).clone();
		if name.is_empty() {
			new_event_name_error_signal.set(String::from("Event must have a name"));
			return;
		}
		if used_names_signal.get().contains(&name) {
			new_event_name_error_signal.set(String::from("This name is already in use."));
			return;
		}
		new_event_name_error_signal.modify().clear();

		let formatted_time = new_event_time_signal.get();
		let start_time = match parse_time_field_value(&formatted_time) {
			Ok(time) => time,
			Err(error) => {
				new_event_time_error_signal.set(format!("Invalid time: {}", error));
				return;
			}
		};
		new_event_time_error_signal.modify().clear();

		let editor_link_format = (*new_event_editor_link_format_signal.get()).clone();
		let first_tab_name = (*new_event_first_tab_name_signal.get()).clone();

		new_event_name_signal.modify().clear();
		new_event_time_signal.set(format!("{}", Utc::now().format(ISO_DATETIME_FORMAT_STRING)));
		let new_event = Event {
			id: String::new(),
			name,
			start_time,
			editor_link_format,
			first_tab_name,
		};

		let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEventsUpdate(
			AdminEventUpdate::UpdateEvent(new_event),
		)));
		let message_json = match serde_json::to_string(&message) {
			Ok(msg) => msg,
			Err(error) => {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to serialize new event message.",
					error,
				));
				return;
			}
		};

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send new event message.", error));
			}
		});
	};

	view! {
		ctx,
		h1 { "Manage Events" }
		div(id="admin_manage_events") {
			div(class="admin_manage_events_row admin_manage_events_headers") {
				div { "Name" }
				div { "Start Time (UTC)" }
				div { "Editor Link Format" }
				div { "First Tab Name" }
				div { }
			}
			Keyed(
				iterable=all_events,
				key=|event| event.id.clone(),
				view=move |ctx, event| {
					let name_signal = create_signal(ctx, event.name.clone());
					let name_error_signal = create_signal(ctx, String::new());
					let time_signal = create_signal(ctx, format!("{}", event.start_time.format(ISO_DATETIME_FORMAT_STRING)));
					let time_error_signal = create_signal(ctx, String::new());
					let editor_link_format_signal = create_signal(ctx, event.editor_link_format.clone());
					let first_tab_name_signal = create_signal(ctx, event.first_tab_name.clone());

					let submit_handler = move |web_event: WebEvent| {
						web_event.prevent_default();

						let name = (*name_signal.get()).clone();
						if name.is_empty() {
							name_error_signal.set(String::from("Event must have a name"));
							return;
						}
						for list_event in data.all_events.get().iter() {
							if event.id != list_event.id && event.name == list_event.name {
								name_error_signal.set(String::from("This name is already in use."));
								return;
							}
						}
						name_error_signal.modify().clear();

						let formatted_time = time_signal.get();
						let start_time = match parse_time_field_value(&formatted_time) {
							Ok(time) => time,
							Err(error) => {
								time_error_signal.set(format!("Invalid time: {}", error));
								return;
							}
						};
						time_error_signal.modify().clear();

						let editor_link_format = (*editor_link_format_signal.get()).clone();
						let first_tab_name = (*first_tab_name_signal.get()).clone();

						let updated_event = Event { id: event.id.clone(), name, start_time, editor_link_format, first_tab_name };
						let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEventsUpdate(AdminEventUpdate::UpdateEvent(updated_event))));
						let message_json = match serde_json::to_string(&message) {
							Ok(msg) => msg,
							Err(error) => {
								let data: &DataSignals = use_context(ctx);
								data.errors.modify().push(ErrorData::new_with_error("Failed to serialize event update message.", error));
								return;
							}
						};
						spawn_local_scoped(ctx, async move {
							let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
							let mut ws = ws_context.lock().await;

							if let Err(error) = ws.send(Message::Text(message_json)).await {
								let data: &DataSignals = use_context(ctx);
								data.errors.modify().push(ErrorData::new_with_error("Failed to send event update message.", error));
							}
						});
					};

					view! {
						ctx,
						form(class="admin_manage_events_row", on:submit=submit_handler) {
							div {
								input(bind:value=name_signal, class=if name_error_signal.get().is_empty() { "" } else { "error" }, title=*name_error_signal.get())
							}
							div {
								input(type="datetime-local", step=1, bind:value=time_signal, class=if time_error_signal.get().is_empty() { "" } else { "error" }, title=*time_error_signal.get())
							}
							div {
								input(bind:value=editor_link_format_signal)
							}
							div {
								input(bind:value=first_tab_name_signal)
							}
							div {
								button(type="submit") { "Update" }
							}
						}
					}
				}
			)
			div(class="admin_manage_events_row admin_manage_events_full_header") {
				h2 { "Add New Event" }
			}
			form(class="admin_manage_events_row", on:submit=new_event_submit_handler) {
				div {
					input(bind:value=new_event_name_signal, class=if new_event_name_error_signal.get().is_empty() { "" } else { "error" }, title=*new_event_name_error_signal.get())
				}
				div {
					input(type="datetime-local", step=1, bind:value=new_event_time_signal, class=if new_event_time_error_signal.get().is_empty() { "" } else { "error" }, title=*new_event_time_error_signal.get())
				}
				div {
					input(bind:value=new_event_editor_link_format_signal)
				}
				div {
					input(bind:value=new_event_first_tab_name_signal)
				}
				div {
					button(type="submit") { "Add event" }
				}
			}
		}
	}
}

#[component]
pub fn AdminManageEventsView<G: Html>(ctx: Scope<'_>) -> View<G> {
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
		Suspense(fallback=view!{ ctx, "Loading events..." }) {
			AdminManageEventsLoadedView
		}
	}
}
