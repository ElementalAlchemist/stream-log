use crate::entry_utils::{parse_time_field_value, ISO_DATETIME_FORMAT_STRING};
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashMap;
use stream_log_shared::messages::admin::AdminEventLogTabsUpdate;
use stream_log_shared::messages::event_log::EventLogTab;
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
async fn AdminManageEventLogTabsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscription_result = {
		let subscriptions = vec![SubscriptionType::AdminEvents, SubscriptionType::AdminEventLogTabs];
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager.set_subscriptions(subscriptions, &mut ws).await
	};
	if let Err(error) = add_subscription_result {
		data.errors
			.modify()
			.push(ErrorData::new_with_error("Failed to subscribe for admin tabs", error));
	}

	let all_events = create_memo(ctx, || (*data.all_events.get()).clone());
	let all_tabs = create_memo(ctx, || (*data.all_event_log_tabs.get()).clone());

	let selected_event: &Signal<Option<Event>> = create_signal(ctx, None);
	let entered_event_name = create_signal(ctx, String::new());

	let events_by_name_index = create_memo(ctx, || {
		let name_index: HashMap<String, Event> = all_events
			.get()
			.iter()
			.map(|event| (event.name.clone(), event.clone()))
			.collect();
		name_index
	});

	let entered_event_error = create_memo(ctx, || {
		let event_name = entered_event_name.get();
		if event_name.is_empty() || events_by_name_index.get().contains_key(&*event_name) {
			String::new()
		} else {
			String::from("Entered name is not the name of an event")
		}
	});

	let event_form_handler = |event: WebEvent| {
		event.prevent_default();

		let entered_name = entered_event_name.get();
		if entered_name.is_empty() {
			selected_event.set(None);
			return;
		}

		let events_by_name_index = events_by_name_index.get();
		let matching_event = events_by_name_index.get(&*entered_name);
		if let Some(event) = matching_event {
			selected_event.set(Some(event.clone()));
		}
	};

	let current_event_tabs = create_memo(ctx, || {
		let tabs = all_tabs.get();
		let selected_event = selected_event.get();

		match selected_event.as_ref() {
			Some(event) => tabs
				.iter()
				.filter(|(tab_event, _)| *tab_event == *event)
				.map(|(_, tab)| tab.clone())
				.collect(),
			None => Vec::new(),
		}
	});

	let new_tab_name_entry = create_signal(ctx, String::new());
	let new_tab_time_entry = create_signal(ctx, String::new());
	let new_tab_error = create_memo(ctx, || {
		let new_tab_name = new_tab_name_entry.get();
		let selected_event = selected_event.get();
		let Some(selected_event) = selected_event.as_ref() else {
			return String::new();
		};
		if new_tab_name.is_empty() {
			String::new()
		} else if all_tabs
			.get()
			.iter()
			.any(|(event, tab)| event.id == selected_event.id && tab.name == *new_tab_name)
		{
			String::from("Already the name of an event")
		} else {
			String::new()
		}
	});

	let new_tab_add_handler = move |event: WebEvent| {
		event.prevent_default();

		let selected_event = selected_event.get();
		let Some(selected_event) = selected_event.as_ref() else {
			return;
		};
		let selected_event = selected_event.clone();

		let name = (*new_tab_name_entry.get()).clone();
		let start_time = new_tab_time_entry.get();
		let Ok(start_time) = parse_time_field_value(&start_time) else {
			return;
		};

		let new_tab = EventLogTab {
			id: String::new(),
			name,
			start_time,
		};

		new_tab_name_entry.set(String::new());
		new_tab_time_entry.set(String::new());

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message =
				FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEventLogTabsUpdate(
					AdminEventLogTabsUpdate::AddTab(selected_event, new_tab),
				)));
			let message_json = match serde_json::to_string(&message) {
				Ok(data) => data,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize new event log tab message.",
						error,
					));
					return;
				}
			};

			let send_result = ws.send(Message::Text(message_json)).await;
			if let Err(error) = send_result {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to send new event log tab message.",
					error,
				));
			}
		});
	};

	view! {
		ctx,
		datalist(id="list_all_events") {
			Keyed(
				iterable=all_events,
				key=|event| event.id.clone(),
				view=|ctx, event| {
					view! {
						ctx,
						option(value=&event.name)
					}
				}
			)
		}
		form(id="admin_tabs_event_selection", on:submit=event_form_handler) {
			input(list="list_all_events", bind:value=entered_event_name, title=entered_event_error.get(), class=if entered_event_error.get().is_empty() { "" } else { "error" })
			button(type="submit") { "Load Event" }
		}
		div(id="admin_tabs_list") {
			Keyed(
				iterable=current_event_tabs,
				key=|tab| tab.id.clone(),
				view=|ctx, tab| {
					let tab_name_entry = create_signal(ctx, tab.name.clone());
					let tab_time_entry = create_signal(ctx, format!("{}", tab.start_time.format(ISO_DATETIME_FORMAT_STRING)));
					let edit_tab_name_handler = {
						let tab = tab.clone();
						move |event: WebEvent| {
							event.prevent_default();

							let name = (*tab_name_entry.get()).clone();
							let start_time = tab_time_entry.get();
							let start_time = parse_time_field_value(&start_time);
							let Ok(start_time) = start_time else { return; };

							let updated_tab = EventLogTab { id: tab.id.clone(), name, start_time };

							spawn_local_scoped(ctx, async move {
								let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEventLogTabsUpdate(AdminEventLogTabsUpdate::UpdateTab(updated_tab))));
								let message_json = match serde_json::to_string(&message) {
									Ok(data) => data,
									Err(error) => {
										let data: &DataSignals = use_context(ctx);
										data.errors.modify().push(ErrorData::new_with_error("Failed to serialize event log tab update.", error));
										return;
									}
								};
								let send_result = ws.send(Message::Text(message_json)).await;
								if let Err(error) = send_result {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to send event log tab update.", error));
								}
							});
						}
					};

					let tab_delete_handler = move |_event: WebEvent| {
						let tab = tab.clone();
						spawn_local_scoped(ctx, async move {
							let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
							let mut ws = ws_context.lock().await;

							let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEventLogTabsUpdate(AdminEventLogTabsUpdate::DeleteTab(tab.clone()))));
							let message_json = match serde_json::to_string(&message) {
								Ok(data) => data,
								Err(error) => {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to serialize event log tab removal.", error));
									return;
								}
							};
							let send_result = ws.send(Message::Text(message_json)).await;
							if let Err(error) = send_result {
								let data: &DataSignals = use_context(ctx);
								data.errors.modify().push(ErrorData::new_with_error("Failed to send event log tab removal.", error));
							}
						});
					};

					view! {
						ctx,
						form(class="admin_tabs_tab", on:submit=edit_tab_name_handler) {
							div {
								input(bind:value=tab_name_entry)
							}
							div {
								input(type="datetime-local", bind:value=tab_time_entry)
							}
							div {
								button(type="submit") { "Update" }
							}
							div {
								button(type="button", on:click=tab_delete_handler) { "Delete" }
							}
						}
					}
				}
			)
		}
		(if selected_event.get().is_some() {
			view! {
				ctx,
				form(id="admin_tabs_add_tab", on:submit=new_tab_add_handler) {
					input(placeholder="Tab name", bind:value=new_tab_name_entry, title=new_tab_error.get(), class=if new_tab_error.get().is_empty() { "" } else { "error" })
					input(type="datetime-local", bind:value=new_tab_time_entry)
					button(type="submit", disabled=!new_tab_error.get().is_empty()) { "Add Tab" }
				}
			}
		} else {
			view! { ctx, }
		})
	}
}

#[component]
pub fn AdminManageEventLogTabsView<G: Html>(ctx: Scope<'_>) -> View<G> {
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
				navigate("/");
			});
			return view! { ctx, };
		}
	}

	view! {
		ctx,
		Suspense(
			fallback=view! { ctx, "Loading event log tabs..." }
		) {
			AdminManageEventLogTabsLoadedView
		}
	}
}
