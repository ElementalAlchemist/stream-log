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
use stream_log_shared::messages::admin::AdminEventLogSectionsUpdate;
use stream_log_shared::messages::event_log::EventLogSection;
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
async fn AdminManageEventLogSectionsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscription_result = {
		let subscriptions = vec![SubscriptionType::AdminEvents, SubscriptionType::AdminEventLogSections];
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager.set_subscriptions(subscriptions, &mut ws).await
	};
	if let Err(error) = add_subscription_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Failed to subscribe for admin sections",
			error,
		));
	}

	let all_events = create_memo(ctx, || (*data.all_events.get()).clone());
	let all_sections = create_memo(ctx, || (*data.all_event_log_sections.get()).clone());

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

	let current_event_sections = create_memo(ctx, || {
		let sections = all_sections.get();
		let selected_event = selected_event.get();

		match selected_event.as_ref() {
			Some(event) => sections
				.iter()
				.filter(|(section_event, _)| *section_event == *event)
				.map(|(_, section)| section.clone())
				.collect(),
			None => Vec::new(),
		}
	});

	let new_section_name_entry = create_signal(ctx, String::new());
	let new_section_time_entry = create_signal(ctx, String::new());
	let new_section_error = create_memo(ctx, || {
		let new_section_name = new_section_name_entry.get();
		let selected_event = selected_event.get();
		let Some(selected_event) = selected_event.as_ref() else { return String::new(); };
		if new_section_name.is_empty() {
			String::new()
		} else if all_sections
			.get()
			.iter()
			.any(|(event, section)| event.id == selected_event.id && section.name == *new_section_name)
		{
			String::from("Already the name of an event")
		} else {
			String::new()
		}
	});

	let new_section_add_handler = move |event: WebEvent| {
		event.prevent_default();

		let selected_event = selected_event.get();
		let Some(selected_event) = selected_event.as_ref() else { return; };
		let selected_event = selected_event.clone();

		let name = (*new_section_name_entry.get()).clone();
		let start_time = new_section_time_entry.get();
		let Ok(start_time) = parse_time_field_value(&start_time) else { return; };

		let new_section = EventLogSection {
			id: String::new(),
			name,
			start_time,
		};

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(
				SubscriptionTargetUpdate::AdminEventLogSectionsUpdate(AdminEventLogSectionsUpdate::AddSection(
					selected_event,
					new_section,
				)),
			));
			let message_json = match serde_json::to_string(&message) {
				Ok(data) => data,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize new event log section message.",
						error,
					));
					return;
				}
			};

			let send_result = ws.send(Message::Text(message_json)).await;
			if let Err(error) = send_result {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to send new event log section message.",
					error,
				));
			}
		});
	};

	view! {
		ctx,
		form(id="admin_sections_event_selection", on:submit=event_form_handler) {
			input(bind:value=entered_event_name, title=entered_event_error.get(), class=if entered_event_error.get().is_empty() { "" } else { "error" })
			button(type="submit") { "Load Event" }
		}
		div(id="admin_sections_list") {
			Keyed(
				iterable=current_event_sections,
				key=|section| section.id.clone(),
				view=|ctx, section| {
					let section_name_entry = create_signal(ctx, section.name.clone());
					let section_time_entry = create_signal(ctx, format!("{}", section.start_time.format(ISO_DATETIME_FORMAT_STRING)));
					let edit_section_name_handler = {
						let section = section.clone();
						move |event: WebEvent| {
							event.prevent_default();

							let name = (*section_name_entry.get()).clone();
							let start_time = section_time_entry.get();
							let start_time = parse_time_field_value(&start_time);
							let Ok(start_time) = start_time else { return; };

							let updated_section = EventLogSection { id: section.id.clone(), name, start_time };

							spawn_local_scoped(ctx, async move {
								let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEventLogSectionsUpdate(AdminEventLogSectionsUpdate::UpdateSection(updated_section))));
								let message_json = match serde_json::to_string(&message) {
									Ok(data) => data,
									Err(error) => {
										let data: &DataSignals = use_context(ctx);
										data.errors.modify().push(ErrorData::new_with_error("Failed to serialize event log section update.", error));
										return;
									}
								};
								let send_result = ws.send(Message::Text(message_json)).await;
								if let Err(error) = send_result {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to send event log section update.", error));
								}
							});
						}
					};

					let section_delete_handler = move |_event: WebEvent| {
						let section = section.clone();
						spawn_local_scoped(ctx, async move {
							let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
							let mut ws = ws_context.lock().await;

							let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEventLogSectionsUpdate(AdminEventLogSectionsUpdate::DeleteSection(section.clone()))));
							let message_json = match serde_json::to_string(&message) {
								Ok(data) => data,
								Err(error) => {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to serialize event log section removal.", error));
									return;
								}
							};
							let send_result = ws.send(Message::Text(message_json)).await;
							if let Err(error) = send_result {
								let data: &DataSignals = use_context(ctx);
								data.errors.modify().push(ErrorData::new_with_error("Failed to send event log section removal.", error));
							}
						});
					};

					view! {
						ctx,
						form(class="admin_sections_section", on:submit=edit_section_name_handler) {
							div {
								input(bind:value=section_name_entry)
							}
							div {
								input(type="datetime-local", bind:value=section_time_entry)
							}
							div {
								button(type="submit") { "Update" }
							}
							div {
								button(type="button", on:click=section_delete_handler) { "Delete" }
							}
						}
					}
				}
			)
		}
		(if selected_event.get().is_some() {
			view! {
				ctx,
				form(id="admin_sections_add_section", on:submit=new_section_add_handler) {
					input(placeholder="Section name", bind:value=new_section_name_entry, title=new_section_error.get(), class=if new_section_error.get().is_empty() { "" } else { "error" })
					input(type="datetime-local", bind:value=new_section_time_entry)
					button(type="submit", disabled=!new_section_error.get().is_empty()) { "Add Section" }
				}
			}
		} else {
			view! { ctx, }
		})
	}
}

#[component]
pub fn AdminManageEventLogSectionsView<G: Html>(ctx: Scope<'_>) -> View<G> {
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
			fallback=view! { ctx, "Loading event log sections..." }
		) {
			AdminManageEventLogSectionsLoadedView
		}
	}
}
