use crate::pages::error::{ErrorData, ErrorView};
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::admin::AdminAction;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AdminManageEditorsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<WebSocket> = use_context(ctx);
	let mut ws = ws_context.lock().await;

	let user_request = RequestMessage::Admin(AdminAction::ListUsers);
	let user_request_json = match serde_json::to_string(&user_request) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to serialize user list request",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};
	if let Err(error) = ws.send(Message::Text(user_request_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error(
			"Failed to send user list request",
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

	let users_response: DataMessage<Vec<UserData>> = match read_websocket(&mut ws).await {
		Ok(data) => data,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to receive user list response",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let events_response: DataMessage<Vec<Event>> = match read_websocket(&mut ws).await {
		Ok(data) => data,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to receive events list response",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let users = match users_response {
		Ok(users) => users,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"A server error occurred processing the user list",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let events = match events_response {
		Ok(events) => events,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"A server error occurred processing the events list",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let users_signal = create_signal(ctx, users);
	let events_signal = create_signal(ctx, events);

	let event_entry = create_signal(ctx, String::new());
	let current_event: &Signal<Option<Event>> = create_signal(ctx, None);
	let event_editors: &Signal<Vec<UserData>> = create_signal(ctx, Vec::new());
	let event_name_index = create_memo(ctx, || {
		let name_index: HashMap<String, Event> = events_signal
			.get()
			.iter()
			.map(|event| (event.name.clone(), event.clone()))
			.collect();
		name_index
	});
	let event_entry_error: &Signal<Option<String>> = create_signal(ctx, None);
	let non_editor_users_signal = create_memo(ctx, || {
		let event_user_ids: HashSet<String> = event_editors.get().iter().map(|editor| editor.id.clone()).collect();
		let non_editor_users: Vec<UserData> = users_signal
			.get()
			.iter()
			.filter(|editor| !event_user_ids.contains(&editor.id))
			.cloned()
			.collect();
		non_editor_users
	});
	let non_editor_users_name_index = create_memo(ctx, || {
		let non_editor_map: HashMap<String, UserData> = non_editor_users_signal
			.get()
			.iter()
			.map(|user| (user.username.clone(), user.clone()))
			.collect();
		non_editor_map
	});

	create_effect(ctx, move || {
		event_editors.track();
		spawn_local_scoped(ctx, async move {
			let Some(current_event) = (*current_event.get()).clone() else { return; };
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let update_message = RequestMessage::Admin(AdminAction::SetEditorsForEvent(
				current_event,
				(*event_editors.get()).clone(),
			));
			let update_message_json = match serde_json::to_string(&update_message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize editor update",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(update_message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error("Failed to send editor update", error)));
				navigate("/error");
			}
		});
	});

	let add_user_entry = create_signal(ctx, String::new());
	let add_user_error: &Signal<Option<String>> = create_signal(ctx, None);

	let event_selection_handler = move |event: WebEvent| {
		event.prevent_default();

		current_event.set(None);
		event_editors.modify().clear();
		add_user_entry.modify().clear();

		let new_event_name = event_entry.get();
		if new_event_name.is_empty() {
			event_entry_error.set(None);
			return;
		}

		let name_index = event_name_index.get();
		let new_event = name_index.get(&*new_event_name);
		let new_event = match new_event {
			Some(event) => event.clone(),
			None => {
				event_entry_error.set(Some(format!("The event {} doesn't exist.", *new_event_name)));
				return;
			}
		};
		event_entry_error.set(None);

		spawn_local_scoped(ctx, async move {
			let editors_request = RequestMessage::Admin(AdminAction::ListEditorsForEvent(new_event.clone()));
			let editors_request_json = match serde_json::to_string(&editors_request) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize editor list request",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			if let Err(error) = ws.send(Message::Text(editors_request_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send editor list request",
					error,
				)));
				navigate("/error");
				return;
			}

			let editors_response: DataMessage<Vec<UserData>> = match read_websocket(&mut ws).await {
				Ok(response) => response,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to receive editor list request",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			let editors = match editors_response {
				Ok(editors) => editors,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"A server error occurred generating the editor list",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			event_editors.set(editors);
			current_event.set(Some(new_event));
		});
	};

	let add_user_handler = |event: WebEvent| {
		event.prevent_default();

		add_user_error.set(None);

		let new_editor_name = add_user_entry.get();
		if new_editor_name.is_empty() {
			return;
		}

		let name_index = non_editor_users_name_index.get();
		let new_editor = name_index.get(&*new_editor_name);
		match new_editor {
			Some(user) => {
				event_editors.modify().push(user.clone());
				add_user_entry.modify().clear();
			}
			None => add_user_error.set(Some(format!(
				"{} is not a user or is already an editor.",
				*new_editor_name
			))),
		}
	};

	view! {
		ctx,
		datalist(id="add_user_selection") {
			Keyed(
				iterable=non_editor_users_signal,
				key=|user| user.id.clone(),
				view=|ctx, user| view! { ctx, option(value=&user.username) }
			)
		}
		datalist(id="events_selection") {
			Keyed(
				iterable=events_signal,
				key=|event| event.id.clone(),
				view=|ctx, event| view! { ctx, option(value=&event.name) }
			)
		}

		form(id="admin_event_editors_event_selection", on:submit=event_selection_handler) {
			input(
				list="events_selection",
				placeholder="Event name",
				bind:value=event_entry,
				class=if event_entry_error.get().is_some() { "error" } else { "" },
				title=if let Some(error_msg) = event_entry_error.get().as_ref() { error_msg } else { "" }
			)
			button { "Load" }
		}
		(if current_event.get().is_some() {
			view! {
				ctx,
				div(id="admin_event_editors_list") {
					Keyed(
						iterable=event_editors,
						key=|editor| editor.id.clone(),
						view=move |ctx, editor| {
							let remove_click_handler = {
								let editor_id = editor.id.clone();
								move |_event: WebEvent| {
									let mut editors_list = event_editors.modify();
									let editor_index = editors_list.iter().enumerate().find(|(_, check_editor)| editor_id == check_editor.id).map(|(index, _)| index);
									if let Some(index) = editor_index {
										editors_list.remove(index);
									}
								}
							};
							view! {
								ctx,
								div(class="admin_event_editors_list_name") { (editor.username) }
								div(class="admin_event_editors_list_remove") {
									button(type="button", on:click=remove_click_handler) { "Remove" }
								}
							}
						}
					)
				}
				(if non_editor_users_signal.get().is_empty() {
					view! { ctx, }
				} else {
					view! {
						ctx,
						form(id="admin_event_editors_add_editor", on:submit=add_user_handler) {
							input(
								list="add_user_selection",
								placeholder="Add user as editor",
								bind:value=add_user_entry,
								class=if add_user_error.get().is_some() { "error" } else { "" },
								title=if let Some(error_msg) = add_user_error.get().as_ref() { error_msg } else { "" }
							)
							button { "Add" }
						}
					}
				})
			}
		} else {
			view! { ctx, }
		})
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
