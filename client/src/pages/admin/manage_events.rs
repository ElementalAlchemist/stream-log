use crate::pages::error::{ErrorData, ErrorView};
use crate::websocket::read_websocket;
use chrono::NaiveDateTime;
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
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use wasm_bindgen::JsCast;
use web_sys::{Event as WebEvent, HtmlButtonElement, HtmlInputElement};

const ISO_DATETIME_FORMAT_STRING: &str = "%Y-%m-%dT%H:%M:%S";
const INCOMPLETE_DATA_ERROR_MSG: &str = "Both values must be populated for a valid new event.";

#[derive(Clone, Default)]
struct PossibleEventData {
	name: Option<String>,
	start_time: Option<NaiveDateTime>,
}

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
async fn AdminManageEventsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
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
	let updated_values: RcSignal<HashMap<String, PossibleEventData>> = create_rc_signal(HashMap::new());
	let new_values: RcSignal<Vec<PossibleEventData>> = create_rc_signal(Vec::new());
	let next_new_index: Rc<AtomicUsize> = Rc::new(AtomicUsize::new(0));

	let submit_button_ref = create_node_ref(ctx);
	let cancel_button_ref = create_node_ref(ctx);

	let form_submission_handler = {
		let updated_values = updated_values.clone();
		let new_values = new_values.clone();
		move |event: WebEvent| {
			event.prevent_default();

			let submit_button_node: DomNode = submit_button_ref.get();
			let submit_button: HtmlButtonElement = submit_button_node.unchecked_into();
			submit_button.set_disabled(true);

			let cancel_button_node: DomNode = cancel_button_ref.get();
			let cancel_button: HtmlButtonElement = cancel_button_node.unchecked_into();
			cancel_button.set_disabled(true);

			let mut changes: Vec<Event> = Vec::new();
			for (id, event_data) in updated_values.get().iter() {
				let (Some(name), Some(start_time)) = (event_data.name.clone(), event_data.start_time) else {
					submit_button.set_disabled(false);
					cancel_button.set_disabled(false);
					return;
				};
				let new_event = Event {
					id: id.clone(),
					name,
					start_time,
				};
				changes.push(new_event);
			}
			for event_data in new_values.get().iter() {
				let (Some(name), Some(start_time)) = (event_data.name.clone(), event_data.start_time) else {
					submit_button.set_disabled(false);
					cancel_button.set_disabled(false);
					return;
				};
				let new_event = Event {
					id: String::new(),
					name,
					start_time,
				};
				changes.push(new_event);
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
		let new_values = new_values.clone();
		move |_: WebEvent| {
			let index = next_new_index.fetch_add(1, Ordering::AcqRel);
			new_values.modify().push(PossibleEventData::default());
			let id = format!("+{}", index);
			event_list.modify().push(Event {
				id,
				name: String::new(),
				start_time: NaiveDateTime::default(),
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
					th { "Start Time" }
				}
				Indexed(
					iterable=event_list,
					view={
						let updated_values = updated_values.clone();
						move |ctx, event| {
							let input_name_name = format!("event-name-{}", event.id);
							let input_time_name = format!("event-start-{}", event.id);

							let name_field = create_node_ref(ctx);
							let time_field = create_node_ref(ctx);

							let name_field_change_handler = {
								let updated_values = updated_values.clone();
								let id = event.id.clone();
								let new_values = new_values.clone();
								move |change_event: WebEvent| {
									let mut values_map = updated_values.modify();
									let event_target = change_event.target().unwrap();
									let field: &HtmlInputElement = event_target.dyn_ref().unwrap();
									let new_name = field.value();
									let new_name = if new_name.is_empty() { None } else { Some(new_name) };

									let time_field_ref: DomNode = time_field.get();
									let time_field_element: HtmlInputElement = time_field_ref.unchecked_into();
									let time_field_value = time_field_element.value();

									if new_name.is_none() && !time_field_value.is_empty() {
										field.class_list().add_1("input-error").expect("Class changes are valid");
										field.set_title(INCOMPLETE_DATA_ERROR_MSG);
										time_field_element.class_list().remove_1("input-error").expect("Class changes are valid");
										time_field_element.set_title("");
									} else if new_name.is_some() && time_field_value.is_empty() {
										field.class_list().remove_1("input-error").expect("Class changes are valid");
										field.set_title("");
										time_field_element.class_list().add_1("input-error").expect("Class changes are valid");
										time_field_element.set_title(INCOMPLETE_DATA_ERROR_MSG);
									} else {
										field.class_list().remove_1("input-error").expect("Class changes are valid");
										field.set_title("");
										time_field_element.class_list().remove_1("input-error").expect("Class changes are valid");
										time_field_element.set_title("");
									}

									if let Some(index) = id.strip_prefix('+') {
										let index: usize = index.parse().unwrap();
										new_values.modify()[index].name = new_name;
									} else {
										values_map.entry(id.clone()).or_default().name = new_name;
									}
								}
							};

							let time_field_change_handler = {
								let updated_values = updated_values.clone();
								let id = event.id.clone();
								let new_values = new_values.clone();
								move |change_event: WebEvent| {
									let mut values_map = updated_values.modify();
									let event_target = change_event.target().unwrap();
									let field: &HtmlInputElement = event_target.dyn_ref().unwrap();
									field.class_list().remove_1("input-error").expect("Input field classes are valid");
									field.set_title("");
									let field_value = field.value();
									let field_value = if field_value.is_empty() {
										None
									} else {
										match NaiveDateTime::parse_from_str(&field_value, "%Y-%m-%dT%H:%M:%S") {
											Ok(dt) => Some(dt),
											Err(error) => {
												let error_description = format!("Invalid date/time: {}", error);
												field.class_list().add_1("input-error").expect("Input field classes are valid");
												field.set_title(&error_description);
												return;
											}
										}
									};

									let name_field_ref: DomNode = name_field.get();
									let name_field_element: HtmlInputElement = name_field_ref.unchecked_into();
									let name_field_value = name_field_element.value();

									if field_value.is_none() && !name_field_value.is_empty() {
										field.class_list().add_1("input-error").expect("Class changes are valid");
										field.set_title(INCOMPLETE_DATA_ERROR_MSG);
										name_field_element.class_list().remove_1("input-error").expect("Class changes are valid");
										name_field_element.set_title("");
									} else if field_value.is_some() && name_field_value.is_empty() {
										field.class_list().remove_1("input-error").expect("Class changes are valid");
										field.set_title("");
										name_field_element.class_list().add_1("input-error").expect("Class changes are valid");
										name_field_element.set_title(INCOMPLETE_DATA_ERROR_MSG);
									} else {
										field.class_list().remove_1("input-error").expect("Class changes are valid");
										field.set_title("");
										name_field_element.class_list().remove_1("input-error").expect("Class changes are valid");
										name_field_element.set_title("");
									}

									if let Some(index) = id.strip_prefix('+') {
										let index: usize = index.parse().unwrap();
										new_values.modify()[index].start_time = field_value;
									} else {
										values_map.entry(id.clone()).or_default().start_time = field_value;
									}
								}
							};

							let start_time_value = format!("{}", event.start_time.format(ISO_DATETIME_FORMAT_STRING));
							view! {
								ctx,
								tr {
									td { (event.name) }
									td {
										input(type="input", name=input_name_name, on:change=name_field_change_handler, ref=name_field)
									}
									td {
										input(type="datetime-local", step=1, name=input_time_name, value=start_time_value, on:change=time_field_change_handler, ref=time_field)
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

#[component]
pub async fn AdminManageEventsView<G: Html>(ctx: Scope<'_>) -> View<G> {
	view! {
		ctx,
		Suspense(fallback=view!{ ctx, "Loading events..." }) {
			AdminManageEventsLoadedView
		}
	}
}
