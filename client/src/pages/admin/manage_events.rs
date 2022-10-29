use crate::pages::error::error_message_view;
use crate::websocket::read_websocket;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use stream_log_shared::messages::admin::AdminAction;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event as WebEvent, HtmlButtonElement, HtmlInputElement};

async fn get_event_list(ws: &mut WebSocket) -> Result<Vec<Event>, ()> {
	let event_list_message = RequestMessage::Admin(AdminAction::ListEvents);
	let event_list_message_json = match serde_json::to_string(&event_list_message) {
		Ok(msg) => msg,
		Err(error) => {
			sycamore::render(|ctx| {
				error_message_view(
					ctx,
					String::from("Failed to serialize request for event list"),
					Some(error),
				)
			});
			return Err(());
		}
	};
	if let Err(error) = ws.send(Message::Text(event_list_message_json)).await {
		sycamore::render(|ctx| {
			error_message_view(ctx, String::from("Failed to send request for event list"), Some(error))
		});
		return Err(());
	}

	let event_list_response = read_websocket(ws).await;
	let event_list: DataMessage<Vec<Event>> = match event_list_response {
		Ok(resp) => resp,
		Err(error) => {
			sycamore::render(|ctx| error_message_view(ctx, String::from("Failed to receive event list"), Some(error)));
			return Err(());
		}
	};
	match event_list {
		Ok(events) => Ok(events),
		Err(error) => {
			let error_message = format!("Failed to generate the event list: {}", error);
			let no_error: Option<String> = None;
			sycamore::render(|ctx| error_message_view(ctx, error_message, no_error));
			Err(())
		}
	}
}

pub async fn handle_admin_manage_events_page(ws: &mut WebSocket) {
	let event_list = match get_event_list(ws).await {
		Ok(events) => events,
		Err(_) => return,
	};
	let (finish_tx, mut finish_rx) = mpsc::unbounded();

	sycamore::render(|ctx| {
		let event_list = create_signal(ctx, event_list);
		let updated_names: RcSignal<HashMap<String, String>> = create_rc_signal(HashMap::new());
		let new_names: RcSignal<Vec<String>> = create_rc_signal(Vec::new());
		let next_new_index: Rc<AtomicUsize> = Rc::new(AtomicUsize::new(0));

		let submit_button_ref = create_node_ref(ctx);
		let cancel_button_ref = create_node_ref(ctx);

		let form_submission_handler = {
			let updated_names = updated_names.clone();
			let new_names = new_names.clone();
			let finish_tx = finish_tx.clone();
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
				if let Err(error) = finish_tx.unbounded_send(changes) {
					sycamore::render(|ctx| {
						error_message_view(
							ctx,
							String::from("An internal communication error occurred in admin events management"),
							Some(error),
						)
					});
				}
			}
		};

		let cancel_form_handler = move |_: WebEvent| {
			let submit_button_node: DomNode = submit_button_ref.get();
			let submit_button: HtmlButtonElement = submit_button_node.unchecked_into();
			submit_button.set_disabled(true);

			let cancel_button_node: DomNode = cancel_button_ref.get();
			let cancel_button: HtmlButtonElement = cancel_button_node.unchecked_into();
			cancel_button.set_disabled(true);

			if let Err(error) = finish_tx.unbounded_send(Vec::new()) {
				sycamore::render(|ctx| {
					error_message_view(
						ctx,
						String::from("An internal communication error occurred in admin events management"),
						Some(error),
					)
				});
			}
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
	});

	let changes = finish_rx.next().await;
	let changes = match changes {
		Some(c) => c,
		None => {
			sycamore::render(|ctx| {
				let no_error: Option<String> = None;
				error_message_view(
					ctx,
					String::from("Failed to receive internal message on admin events management page"),
					no_error,
				)
			});
			return;
		}
	};

	if !changes.is_empty() {
		let message = RequestMessage::Admin(AdminAction::EditEvents(changes));
		let message_json = match serde_json::to_string(&message) {
			Ok(msg) => msg,
			Err(error) => {
				sycamore::render(|ctx| {
					error_message_view(ctx, String::from("Failed to serialize event name changes"), Some(error))
				});
				return;
			}
		};
		if let Err(error) = ws.send(Message::Text(message_json)).await {
			sycamore::render(|ctx| {
				error_message_view(ctx, String::from("Failed to send event name changes"), Some(error))
			});
		}
	}
}
