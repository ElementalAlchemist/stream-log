// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::color_utils::rgb_str_from_color;
use crate::page_utils::set_page_title;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use crate::websocket::WebSocketSendStream;
use futures::lock::Mutex;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::admin::{AdminEventEditorUpdate, EditorEventAssociation};
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
async fn AdminManageEditorsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	set_page_title("Manage Editors | Stream Log");

	let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscriptions_result = {
		let subscriptions = vec![
			SubscriptionType::AdminEventEditors,
			SubscriptionType::AdminEvents,
			SubscriptionType::AdminUsers,
		];
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager.set_subscriptions(subscriptions, &mut ws).await
	};
	if let Err(error) = add_subscriptions_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send event editors subscription message.",
			error,
		));
	}

	let all_events = create_memo(ctx, || (*data.all_events.get()).clone());
	let all_users = create_memo(ctx, || (*data.all_users.get()).clone());

	let selected_event: &Signal<Option<Event>> = create_signal(ctx, None);
	let event_input = create_signal(ctx, String::new());
	let event_input_error = create_signal(ctx, String::new());

	let event_name_index = create_memo(ctx, || {
		let name_index: HashMap<String, Event> = data
			.all_events
			.get()
			.iter()
			.map(|event| (event.name.clone(), event.clone()))
			.collect();
		name_index
	});

	let current_event_editor_ids = create_memo(ctx, || {
		let event_editors = data.event_editors.get();
		let current_event = match selected_event.get().as_ref() {
			Some(event) => event.clone(),
			None => return HashSet::new(),
		};

		let mut users: HashSet<String> = HashSet::new();
		for event_editor in event_editors
			.iter()
			.filter(|association| association.event.id == current_event.id)
		{
			users.insert(event_editor.editor.id.clone());
		}
		users
	});

	let event_selection_handler = |event: WebEvent| {
		event.prevent_default();

		let name_index = event_name_index.get();
		let entered_name = event_input.get();

		if entered_name.is_empty() {
			event_input_error.set(String::new());
			selected_event.set(None);
			return;
		}

		let entered_event = name_index.get(&*entered_name);
		match entered_event {
			Some(event) => {
				selected_event.set(Some(event.clone()));
				event_input_error.set(String::new());
			}
			None => {
				selected_event.set(None);
				event_input_error.set(String::from("Entered name doesn't match the name of an event"));
			}
		}
	};

	view! {
		ctx,
		datalist(id="events_selection") {
			Keyed(
				iterable=all_events,
				key=|event| event.id.clone(),
				view=|ctx, event| view! { ctx, option(value=&event.name) }
			)
		}

		form(id="admin_event_editors_event_selection", on:submit=event_selection_handler) {
			input(
				list="events_selection",
				placeholder="Event name",
				bind:value=event_input
			)
			button { "Select Event" }
			span(class="input_error") { (event_input_error.get()) }
		}
		table(id="admin_event_editors_list") {
			(if selected_event.get().is_some() {
				view! {
					ctx,
					Keyed(
						iterable=all_users,
						key=|user| user.id.clone(),
						view=move |ctx, user| {
							let is_editor = create_memo(ctx, {
								let user_id = user.id.clone();
								move || current_event_editor_ids.get().contains(&user_id)
							});

							let toggle_user_editor = {
								let user = user.clone();
								move |_event: WebEvent| {
									let selected_event = match selected_event.get().as_ref() {
										Some(event) => event.clone(),
										None => return
									};
									let editor_event_association = EditorEventAssociation { event: selected_event, editor: user.clone() };
									let editor_update_message = if *is_editor.get() {
										AdminEventEditorUpdate::RemoveEditor(editor_event_association)
									} else {
										AdminEventEditorUpdate::AddEditor(editor_event_association)
									};
									let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEventEditorsUpdate(editor_update_message)));

									spawn_local_scoped(ctx, async move {
										let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
										let mut ws = ws_context.lock().await;

										let message_json = match serde_json::to_string(&message) {
											Ok(msg) => msg,
											Err(error) => {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to serialize admin editor update.", error));
												return;
											}
										};

										let send_result = ws.send(Message::Text(message_json)).await;
										if let Err(error) = send_result {
											let data: &DataSignals = use_context(ctx);
											data.errors.modify().push(ErrorData::new_with_error("Failed to send admin editor update.", error));
										}
									});
								}
							};

							let user_color_style = format!("color: {}", rgb_str_from_color(user.color));

							view! {
								ctx,
								tr {
									td(style=user_color_style) { (user.username) }
									td {
										(if *is_editor.get() {
											"✔️"
										} else {
											""
										})
									}
									td {
										button(type="button", on:click=toggle_user_editor) {
											(if *is_editor.get() {
												"Remove"
											} else {
												"Add"
											})
										}
									}
								}
							}
						}
					)
				}
			} else {
				view! { ctx, }
			})
		}
	}
}

#[component]
pub fn AdminManageEditorsView<G: Html>(ctx: Scope<'_>) -> View<G> {
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
		Suspense(fallback=view! { ctx, "Loading editors manager..." }) {
			AdminManageEditorsLoadedView
		}
	}
}
