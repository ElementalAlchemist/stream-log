use crate::pages::error::{ErrorData, ErrorView};
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use stream_log_shared::messages::admin::AdminAction;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore_router::navigate;
use wasm_bindgen::JsCast;
use web_sys::{Event as WebEvent, HtmlButtonElement, HtmlInputElement};

async fn get_event_list(ctx: Scope<'_>) -> Result<Vec<Event>, ()> {
	let ws_context: &Mutex<WebSocket> = use_context(ctx);
	let mut ws = ws_context.lock().await;

	let event_list_message = RequestMessage::Admin(AdminAction::ListEvents);
	let event_list_message_json = match serde_json::to_string(&event_list_message) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to serialize request for event list"),
				error,
			)));
			return Err(());
		}
	};
	if let Err(error) = ws.send(Message::Text(event_list_message_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error(
			String::from("Failed to send request for event list"),
			error,
		)));
		return Err(());
	}

	let event_list_response = read_websocket(&mut ws).await;
	let event_list: DataMessage<Vec<Event>> = match event_list_response {
		Ok(resp) => resp,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to receive event list"),
				error,
			)));
			return Err(());
		}
	};
	match event_list {
		Ok(events) => Ok(events),
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to generate the event list"),
				error,
			)));
			Err(())
		}
	}
}

#[component]
pub async fn AdminManageEventsView<G: Html>(ctx: Scope<'_>) -> View<G> {
	log::debug!("Activating admin events management page");

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

	let Ok(event_list) = get_event_list(ctx).await else {
		return view! { ctx, ErrorView };
	};

	let event_list = create_signal(ctx, event_list);
	let updated_names: RcSignal<HashMap<String, String>> = create_rc_signal(HashMap::new());
	let new_names: RcSignal<Vec<String>> = create_rc_signal(Vec::new());
	let next_new_index: Rc<AtomicUsize> = Rc::new(AtomicUsize::new(0));

	let submit_button_ref = create_node_ref(ctx);
	let cancel_button_ref = create_node_ref(ctx);

	let form_submission_handler = {
		let updated_names = updated_names.clone();
		let new_names = new_names.clone();
		move |event: WebEvent| {
			event.prevent_default();

			let submit_button_node: DomNode = submit_button_ref.get();
			let submit_button: HtmlButtonElement = submit_button_node.unchecked_into();
			submit_button.set_disabled(true);

			let cancel_button_node: DomNode = cancel_button_ref.get();
			let cancel_button: HtmlButtonElement = cancel_button_node.unchecked_into();
			cancel_button.set_disabled(true);

			let mut changes: Vec<Event> = Vec::new();
			for (id, name) in updated_names.get().iter() {
				let new_event = Event {
					id: id.clone(),
					name: name.clone(),
				};
				changes.push(new_event);
			}
			for name in new_names.get().iter() {
				if !name.is_empty() {
					let new_event = Event {
						id: String::new(),
						name: name.clone(),
					};
					changes.push(new_event);
				}
			}

			if changes.is_empty() {
				navigate("/");
			} else {
				let message = RequestMessage::Admin(AdminAction::EditEvents(changes));
				let message_json = match serde_json::to_string(&message) {
					Ok(msg) => msg,
					Err(error) => {
						let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
						error_signal.set(Some(ErrorData::new_with_error(
							String::from("Failed to serialize event name changes"),
							error,
						)));
						navigate("/error");
						return;
					}
				};

				spawn_local_scoped(ctx, async move {
					let ws_context: &Mutex<WebSocket> = use_context(ctx);
					let mut ws = ws_context.lock().await;
					match ws.send(Message::Text(message_json)).await {
						Ok(_) => navigate("/"),
						Err(error) => {
							let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
							error_signal.set(Some(ErrorData::new_with_error(
								String::from("Failed to send event name changes"),
								error,
							)));
							navigate("/error");
						}
					}
				});
			}
		}
	};

	let cancel_form_handler = move |_: WebEvent| {
		navigate("/");
	};

	let add_row_handler = {
		let next_new_index = Rc::clone(&next_new_index);
		let new_names = new_names.clone();
		move |_: WebEvent| {
			let index = next_new_index.fetch_add(1, Ordering::AcqRel);
			new_names.modify().push(String::new());
			let id = format!("+{}", index);
			event_list.modify().push(Event {
				id,
				name: String::new(),
			});
		}
	};

	view! {
		ctx,
		h1 { "Manage Events" }
		form(id="admin_manage_events", on:submit=form_submission_handler) {
			table {
				tr {
					th { "Name" }
					th { "New Name" }
				}
				Indexed(
					iterable=event_list,
					view={
						let updated_names = updated_names.clone();
						move |ctx, event| {
							let updated_names = updated_names.clone();
							let input_name = format!("event-name-{}", event.id);
							let field_change_handler = {
								let id = event.id.clone();
								let new_names = new_names.clone();
								move |change_event: WebEvent| {
									let mut names_map = updated_names.modify();
									let event_target = change_event.target().unwrap();
									let field: &HtmlInputElement = event_target.dyn_ref().unwrap();
									let new_value = field.value();
									if let Some(index) = id.strip_prefix('+') {
										let index: usize = index.parse().unwrap();
										new_names.modify()[index] = new_value;
									} else if new_value.is_empty() {
										names_map.remove(&id.clone());
									} else {
										names_map.insert(id.clone(), new_value);
									}
								}
							};
							view! {
								ctx,
								tr {
									td { (event.name) }
									td {
										input(type="input", name=input_name, on:change=field_change_handler)
									}
								}
							}
						}
					}
				)
			}
			div {
				button(id="admin_manage_events_submit", ref=submit_button_ref) { "Update Names" }
				button(type="button", on:click=cancel_form_handler, ref=cancel_button_ref) { "Cancel" }
				button(type="button", id="admin_manage_events_new_row", on:click=add_row_handler) { "Add New Event" }
			}
		}
	}
}
