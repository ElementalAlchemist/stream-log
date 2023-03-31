use crate::event_type_colors::use_white_foreground;
use crate::pages::error::{ErrorData, ErrorView};
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::admin::AdminAction;
use stream_log_shared::messages::entry_types::EntryType;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AdminManageEventTypesForEventsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;

	let event_types_request = RequestMessage::Admin(AdminAction::ListEventTypes);
	let event_types_request_json = match serde_json::to_string(&event_types_request) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to serialize event type list request",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};
	if let Err(error) = ws.send(Message::Text(event_types_request_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error(
			"Failed to send event type list request",
			error,
		)));
		return view! { ctx, ErrorView };
	}

	let events_request = RequestMessage::Admin(AdminAction::ListEvents);
	let events_request_json = match serde_json::to_string(&events_request) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to serialize events list request",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};
	if let Err(error) = ws.send(Message::Text(events_request_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error(
			"Failed to send events list request",
			error,
		)));
		return view! { ctx, ErrorView };
	}

	let event_types_response: DataMessage<Vec<EntryType>> = match read_websocket(&mut ws).await {
		Ok(response) => response,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to receive event type list response",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};
	let events_response: DataMessage<Vec<Event>> = match read_websocket(&mut ws).await {
		Ok(response) => response,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to receive events list response",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let event_types = match event_types_response {
		Ok(resp) => resp,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"A server error occurred getting event types",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};
	let events = match events_response {
		Ok(resp) => resp,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"A server error occurred getting events",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let event_types_signal = create_signal(ctx, event_types);
	let events_signal = create_signal(ctx, events);
	let selected_event_signal: &Signal<Option<Event>> = create_signal(ctx, None);
	let selected_event_available_types_signal: &Signal<Vec<EntryType>> = create_signal(ctx, Vec::new());
	let loading_selected_event = create_signal(ctx, false);

	let entered_event_signal = create_signal(ctx, String::new());
	let entered_event_error_signal = create_signal(ctx, String::new());

	create_effect(ctx, move || {
		if let Some(selected_event) = (*selected_event_signal.get()).clone() {
			spawn_local_scoped(ctx, async move {
				loading_selected_event.set(true);
				let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
				let mut ws = ws_context.lock().await;

				let message = RequestMessage::Admin(AdminAction::ListEventTypesForEvent(selected_event.clone()));
				let message_json = match serde_json::to_string(&message) {
					Ok(msg) => msg,
					Err(error) => {
						let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
						error_signal.set(Some(ErrorData::new_with_error(
							"Failed to serialize event type list request for event",
							error,
						)));
						navigate("/error");
						return;
					}
				};
				if let Err(error) = ws.send(Message::Text(message_json)).await {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to send event type list request for event",
						error,
					)));
					navigate("/error");
					return;
				}

				let event_types_response: DataMessage<Vec<EntryType>> = match read_websocket(&mut ws).await {
					Ok(msg) => msg,
					Err(error) => {
						let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
						error_signal.set(Some(ErrorData::new_with_error(
							"Failed to receive event type list response for event",
							error,
						)));
						navigate("/error");
						return;
					}
				};

				let event_types = match event_types_response {
					Ok(event_types) => event_types,
					Err(error) => {
						let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
						error_signal.set(Some(ErrorData::new_with_error(
							"A server error occurred retrieving the event type list for the event",
							error,
						)));
						navigate("/error");
						return;
					}
				};

				if let Some(current_event) = selected_event_signal.get().as_ref() {
					if current_event.id == selected_event.id {
						selected_event_available_types_signal.set(event_types);
					}
				}
				loading_selected_event.set(false);
			});
		} else {
			selected_event_available_types_signal.modify().clear();
		}
	});

	let update_handler = move |event: WebEvent| {
		event.prevent_default();

		let selected_event_types: Vec<EntryType> = selected_event_available_types_signal.modify().drain(..).collect();
		let event = if let Some(event) = selected_event_signal.modify().take() {
			event
		} else {
			return;
		};

		let message = RequestMessage::Admin(AdminAction::UpdateEventTypesForEvent(event, selected_event_types));
		let message_json = match serde_json::to_string(&message) {
			Ok(msg) => msg,
			Err(error) => {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to serialize event update",
					error,
				)));
				navigate("/error");
				return;
			}
		};

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error("Failed to send event update", error)));
				navigate("/error");
			}
		});
	};

	let switch_event_handler = move |event: WebEvent| {
		event.prevent_default();

		let events_data = events_signal.get();
		let Some(event) = events_data.iter().find(|event| *entered_event_signal.get() == event.name) else {
			entered_event_error_signal.set(String::from("The entered event does not exist"));
			return;
		};

		selected_event_signal.set(Some(event.clone()));
		entered_event_signal.modify().clear();
	};

	let enter_event_name_handler = |_event: WebEvent| {
		entered_event_error_signal.modify().clear();
	};

	let done_handler = |_event: WebEvent| {
		navigate("/");
	};

	view! {
		ctx,
		(if let Some(event) = (*selected_event_signal.get()).clone() {
			if *loading_selected_event.get() {
				view! { ctx, }
			} else {
				view! {
					ctx,
					h1 { (event.name) }
					form(id="admin_event_type_assignment", on:submit=update_handler) {
						div(id="admin_event_type_assignment_grid") {
							Keyed(
								iterable=event_types_signal,
								key=|event_type| event_type.id.clone(),
								view=move |ctx, event_type| {
									let event_has_type = create_memo(ctx, {
										let event_type = event_type.clone();
										move || {
											selected_event_available_types_signal.get().iter().any(|et| event_type.id == et.id)
										}
									});
									let selected_signal = create_signal(ctx, *event_has_type.get());

									create_effect(ctx, {
										let event_type = event_type.clone();
										move || {
											if *selected_signal.get() {
												if !*event_has_type.get() {
													selected_event_available_types_signal.modify().push(event_type.clone());
												}
											} else {
												let mut modify_event_types = selected_event_available_types_signal.modify();
												if let Some(index) = modify_event_types.iter().enumerate().find(|(_, et)| event_type.id == et.id).map(|(index, _)| index) {
													modify_event_types.remove(index);
												}
											}
										}
									});

									let foreground_color = if use_white_foreground(&event_type.color) {
										"#fff"
									} else {
										"#000"
									};
									let name_style = format!("color: {}; background: rgb({}, {}, {})", foreground_color, event_type.color.r, event_type.color.g, event_type.color.b);

									view! {
										ctx,
										div(class="admin_event_type_assignment_row") {
											div(class="admin_event_type_assignment_name", style=name_style) { (event_type.name) }
											div(class="admin_event_type_assignment_available") {
												input(type="checkbox", bind:checked=selected_signal)
											}
										}
									}
								}
							)
						}
						button { "Update" }
					}
				}
			}
		} else {
			view! {
				ctx,
				form(id="admin_event_type_event_selection", on:submit=switch_event_handler) {
					datalist(id="admin_event_type_event_list") {
						Keyed(
							iterable=events_signal,
							key=|event| event.id.clone(),
							view=|ctx, event| {
								view! { ctx, option(value=event.name) }
							}
						)
					}
					input(
						list="admin_event_type_event_list",
						bind:value=entered_event_signal,
						on:change=enter_event_name_handler,
						class=if entered_event_error_signal.get().is_empty() { "" } else { "error" }
					)
					span(class="input_error") { (*entered_event_error_signal.get()) }
					button { "Load" }
				}
				button(on:click=done_handler) { "Done" }
			}
		})
	}
}

#[component]
pub fn AdminManageEventTypesForEventsView<G: Html>(ctx: Scope<'_>) -> View<G> {
	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading event type data..." }) {
			AdminManageEventTypesForEventsLoadedView
		}
	}
}
