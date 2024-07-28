// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::color_utils::rgb_str_from_color;
use crate::entry_type_colors::use_white_foreground;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use crate::websocket::WebSocketSendStream;
use futures::lock::Mutex;
use gloo_net::websocket::Message;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{AdminEntryTypeEventUpdate, EntryTypeEventAssociation};
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
async fn AdminManageEntryTypesForEventsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscriptions_result = {
		let subscriptions = vec![
			SubscriptionType::AdminEvents,
			SubscriptionType::AdminEntryTypes,
			SubscriptionType::AdminEntryTypesEvents,
		];
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager.set_subscriptions(subscriptions, &mut ws).await
	};
	if let Err(error) = add_subscriptions_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send entry types and events subscription message.",
			error,
		));
	}

	let all_events = create_memo(ctx, || (*data.all_events.get()).clone());
	let all_entry_types = create_memo(ctx, || (*data.all_entry_types.get()).clone());

	let selected_event_signal: &Signal<Option<Event>> = create_signal(ctx, None);

	let entered_event_signal = create_signal(ctx, String::new());
	let entered_event_error_signal = create_signal(ctx, String::new());

	let all_events_name_index = create_memo(ctx, || {
		let name_index: HashMap<String, Event> = data
			.all_events
			.get()
			.iter()
			.map(|event| (event.name.clone(), event.clone()))
			.collect();
		name_index
	});

	let switch_event_handler = move |event: WebEvent| {
		event.prevent_default();

		let event_names_index = all_events_name_index.get();
		let Some(event) = event_names_index.get(&*entered_event_signal.get()) else {
			entered_event_error_signal.set(String::from("The entered event does not exist"));
			return;
		};
		entered_event_error_signal.modify().clear();

		selected_event_signal.set(Some(event.clone()));
	};

	view! {
		ctx,
		datalist(id="all_event_names") {
			Keyed(
				iterable=all_events,
				key=|event| event.id.clone(),
				view=|ctx, event| {
					view! {
						ctx,
						option(value=event.name)
					}
				}
			)
		}
		datalist(id="all_entry_type_names") {
			Keyed(
				iterable=all_entry_types,
				key=|entry_type| entry_type.id.clone(),
				view=|ctx, entry_type| {
					view! {
						ctx,
						option(value=entry_type.name)
					}
				}
			)
		}
		form(id="admin_entry_type_assignment_event_selection", on:submit=switch_event_handler) {
			input(bind:value=entered_event_signal, placeholder="Event name", list="all_event_names")
			button(type="submit") { "Load" }
		}
		(if let Some(event) = selected_event_signal.get().as_ref() {
			view! {
				ctx,
				div(id="admin_event_type_assignment_grid") {
					Keyed(
						iterable=all_entry_types,
						key=|entry_type| entry_type.id.clone(),
						view={
							let event = event.clone();
							move |ctx, entry_type| {
								let is_checked = create_memo(ctx, {
									let entry_type_event_associations = data.entry_type_event_associations.clone();
									let entry_type = entry_type.clone();
									let event = event.clone();
									move || entry_type_event_associations.get().iter().any(|association| association.event.id == event.id && association.entry_type.id == entry_type.id)
								});

								let button_handler = {
									let entry_type = entry_type.clone();
									let event = event.clone();
									move |_event: WebEvent| {
										let association = EntryTypeEventAssociation { entry_type: entry_type.clone(), event: event.clone() };
										let message = if *is_checked.get() {
											FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEntryTypesEventsUpdate(AdminEntryTypeEventUpdate::RemoveTypeFromEvent(association))))
										} else {
											FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEntryTypesEventsUpdate(AdminEntryTypeEventUpdate::AddTypeToEvent(association))))
										};
										let message_json = match serde_json::to_string(&message) {
											Ok(msg) => msg,
											Err(error) => {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to serialize entry type/event association update.", error));
												return;
											}
										};

										spawn_local_scoped(ctx, async move {
											let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
											let mut ws = ws_context.lock().await;

											let send_result = ws.send(Message::Text(message_json)).await;
											if let Err(error) = send_result {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to send entry type/event association update.", error));
											}
										});
									}
								};

								let background_color = rgb_str_from_color(entry_type.color);
								let foreground_color = if use_white_foreground(&entry_type.color) { "#fff" } else { "#000" };
								let name_style = format!("color: {}; background: {}; font-weight: 700", foreground_color, background_color);

								view! {
									ctx,
									div(class="admin_event_type_assignment_name", style=name_style) { (entry_type.name) }
									div(class="admin_event_type_assignment_available") {
										(if *is_checked.get() { "✔️" } else { "" })
									}
									div(class="admin_event_type_assignment_modify") {
										button(on:click=button_handler) {
											(if *is_checked.get() {
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
			}
		} else {
			view! { ctx, }
		})
	}
}

#[component]
pub fn AdminManageEntryTypesForEventsView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let user: &Signal<Option<UserData>> = use_context(ctx);
	if let Some(user) = user.get().as_ref() {
		if !user.is_admin {
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
		Suspense(fallback=view! { ctx, "Loading event type data..." }) {
			AdminManageEntryTypesForEventsLoadedView
		}
	}
}
