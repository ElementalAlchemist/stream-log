use crate::pages::error::{ErrorData, ErrorView};
use crate::websocket::read_websocket;
use contrast::contrast;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use rgb::RGB8;
use stream_log_shared::messages::admin::AdminAction;
use stream_log_shared::messages::event_types::EventType;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

const WHITE: RGB8 = RGB8::new(255, 255, 255);
const BLACK: RGB8 = RGB8::new(0, 0, 0);
const DEFAULT_COLOR: &str = "#ffffff";

#[derive(Clone, Copy, Eq, PartialEq)]
enum SelectedIndex {
	NewType,
	Existing(usize),
}

#[component]
async fn AdminManageEventTypesLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<WebSocket> = use_context(ctx);
	let mut ws = ws_context.lock().await;

	let message = RequestMessage::Admin(AdminAction::ListEventTypes);
	let message_json = match serde_json::to_string(&message) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to serialize event types list request"),
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};
	if let Err(error) = ws.send(Message::Text(message_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error(
			String::from("Failed to send event types list request"),
			error,
		)));
		return view! { ctx, ErrorView };
	}

	let event_types_response: DataMessage<Vec<EventType>> = match read_websocket(&mut ws).await {
		Ok(data) => data,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to receive event types list response"),
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let event_types = match event_types_response {
		Ok(event_types) => event_types,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("A server error occurred getting the event types list"),
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let event_types_signal = create_signal(ctx, event_types);
	let selected_event_type_signal: &Signal<Option<SelectedIndex>> = create_signal(ctx, None);

	let entered_name_signal = create_signal(ctx, String::new());
	let entered_color_signal = create_signal(ctx, String::from(DEFAULT_COLOR));
	let entered_name_error_signal = create_signal(ctx, String::new());

	let add_event_type_handler = |_event: WebEvent| {
		selected_event_type_signal.set(Some(SelectedIndex::NewType));
		entered_name_signal.set(String::new());
		entered_color_signal.set(String::from(DEFAULT_COLOR));
	};

	let done_click_handler = |_event: WebEvent| {
		navigate("/");
	};

	view! {
		ctx,
		div(id="admin_event_type_list") {
			Keyed(
				iterable=event_types_signal,
				key=|event_type| event_type.id.clone(),
				view=move |ctx, event_type| {
					let click_handler = {
						let event_type = event_type.clone();
						move |_event: WebEvent| {
							selected_event_type_signal.set(Some(SelectedIndex::Existing(event_types_signal.get().iter().enumerate().find(|(_, et)| et.id == event_type.id).map(|(index, _)| index).unwrap())));
							entered_name_signal.set(event_type.name.clone());

							let color_red = event_type.color.r;
							let color_green = event_type.color.g;
							let color_blue = event_type.color.b;

							let color = format!("#{:02x}{:02x}{:02x}", color_red, color_green, color_blue);
							entered_color_signal.set(color);
						}
					};

					let white_contrast: f64 = contrast(event_type.color, WHITE);
					let black_contrast: f64 = contrast(event_type.color, BLACK);

					let foreground_color = if white_contrast > black_contrast {
						"#fff"
					} else {
						"#000"
					};

					let background_color = format!("rgb({}, {}, {})", event_type.color.r, event_type.color.g, event_type.color.b);

					let style = format!("color: {}; background: {}", foreground_color, background_color);
					view! {
						ctx,
						div(class="admin_event_type click", style=style, on:click=click_handler) { (event_type.name) }
					}
				}
			)
		}

		(if let Some(selected_event_type) = *selected_event_type_signal.get() {
			let form_submission_handler = move |event: WebEvent| {
				event.prevent_default();

				let name = (*entered_name_signal.get()).clone();
				if name.is_empty() {
					entered_name_error_signal.set(String::from("Name cannot be empty."));
					return;
				}
				let color_str = (*entered_color_signal.get()).clone();
				// Assuming a functioning browser color input, we don't have error output for this parsing
				if color_str.len() != 7 {
					return;
				}
				let Some(color_str) = color_str.strip_prefix('#') else {
					return;
				};
				let mut color_chars = color_str.chars();
				let mut color_red = String::new();
				let mut color_green = String::new();
				let mut color_blue = String::new();
				color_red.push(color_chars.next().unwrap());
				color_red.push(color_chars.next().unwrap());
				color_green.push(color_chars.next().unwrap());
				color_green.push(color_chars.next().unwrap());
				color_blue.push(color_chars.next().unwrap());
				color_blue.push(color_chars.next().unwrap());

				let Ok(color_red) = u8::from_str_radix(&color_red, 16) else {
					return;
				};
				let Ok(color_green) = u8::from_str_radix(&color_green, 16) else {
					return;
				};
				let Ok(color_blue) = u8::from_str_radix(&color_blue, 16) else {
					return;
				};

				let color = RGB8::new(color_red, color_green, color_blue);

				let mut event_type_data = match selected_event_type {
					SelectedIndex::NewType => EventType { id: String::new(), name: String::new(), color: WHITE },
					SelectedIndex::Existing(index) => event_types_signal.get()[index].clone()
				};

				event_type_data.name = name;
				event_type_data.color = color;

				selected_event_type_signal.set(None);

				if let SelectedIndex::Existing(index) = selected_event_type {
					event_types_signal.modify()[index] = event_type_data.clone();
				}
				spawn_local_scoped(ctx, async move {
					let ws_context: &Mutex<WebSocket> = use_context(ctx);
					let mut ws = ws_context.lock().await;

					let message = if selected_event_type == SelectedIndex::NewType {
						RequestMessage::Admin(AdminAction::AddEventType(event_type_data.clone()))
					} else {
						RequestMessage::Admin(AdminAction::UpdateEventType(event_type_data.clone()))
					};
					let message_json = match serde_json::to_string(&message) {
						Ok(msg) => msg,
						Err(error) => {
							let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
							error_signal.set(Some(ErrorData::new_with_error(String::from("Failed to serialize event type update"), error)));
							navigate("/error");
							return;
						}
					};
					if let Err(error) = ws.send(Message::Text(message_json)).await {
						let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
						error_signal.set(Some(ErrorData::new_with_error(String::from("Failed to send event type update"), error)));
						navigate("/error");
						return;
					}

					if selected_event_type == SelectedIndex::NewType {
						let id_response: DataMessage<String> = match read_websocket(&mut ws).await {
							Ok(msg) => msg,
							Err(error) => {
								let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
								error_signal.set(Some(ErrorData::new_with_error(String::from("Failed to receive new event type ID"), error)));
								navigate("/error");
								return;
							}
						};
						let id = match id_response {
							Ok(id) => id,
							Err(error) => {
								let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
								error_signal.set(Some(ErrorData::new_with_error(String::from("A server error occurred adding the new event type"), error)));
								navigate("/error");
								return;
							}
						};
						event_type_data.id = id;
						event_types_signal.modify().push(event_type_data);
					}
				});
			};
			let name_field_change_handler = |_event: WebEvent| {
				entered_name_error_signal.modify().clear();
			};
			view! {
				ctx,
				form(id="admin_event_type_edit", on:submit=form_submission_handler) {
					label(for="admin_event_type_edit_name") { "Name" }
					input(id="admin_event_type_edit_name", on:change=name_field_change_handler, bind:value=entered_name_signal, class=if entered_name_error_signal.get().is_empty() { "" } else { "error" })
					input(type="color", bind:value=entered_color_signal)
					button { "Update" }
				}
			}
		} else {
			view! { ctx, }
		})

		div(id="admin_event_type_controls") {
			button(on:click=add_event_type_handler) { "Add Event Type" }
			button(on:click=done_click_handler) { "Done" }
		}
	}
}

#[component]
pub fn AdminManageEventTypesView<G: Html>(ctx: Scope<'_>) -> View<G> {
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
		Suspense(fallback=view! { ctx, "Loading event types data..." }) {
			AdminManageEventTypesLoadedView
		}
	}
}