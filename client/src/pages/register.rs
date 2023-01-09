use super::error::ErrorData;
use crate::color_utils::color_from_rgb_str;
use crate::components::color_input_with_contrast::ColorInputWithContrast;
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::user_register::{
	RegistrationResponse, UserRegistration, UserRegistrationFinalize, UsernameCheckResponse, UsernameCheckStatus,
	USERNAME_LENGTH_LIMIT,
};
use stream_log_shared::messages::DataMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
pub fn RegistrationView<G: Html>(ctx: Scope<'_>) -> View<G> {
	{
		let user_signal: &Signal<Option<UserData>> = use_context(ctx);
		if user_signal.get().is_some() {
			spawn_local_scoped(ctx, async {
				navigate("/");
			});
			return view! { ctx, };
		}
	}

	let username_signal = create_signal(ctx, String::new());
	let username_in_use_signal = create_signal(ctx, false);
	let username_empty_signal = create_memo(ctx, || username_signal.get().is_empty());
	let username_too_long_signal = create_memo(ctx, || username_signal.get().len() > USERNAME_LENGTH_LIMIT);
	let username_field = create_node_ref(ctx);
	let color_signal = create_signal(ctx, String::from("#7f7f7f"));
	let submit_button_ref = create_node_ref(ctx);

	// Username error signal collects all possible kinds of username errors
	let username_error_signal = create_memo(ctx, || {
		*username_in_use_signal.get() || *username_empty_signal.get() || *username_too_long_signal.get()
	});
	// Username error class signal determines what the class of the username field should be based on whether there's an error
	let username_error_class_signal = create_memo(ctx, || if *username_error_signal.get() { "error" } else { "" });

	let form_submission_handler = move |event: WebEvent| {
		event.prevent_default();

		let username = username_signal.get();
		if username.is_empty() {
			return;
		}
		if username.len() > USERNAME_LENGTH_LIMIT {
			return;
		}

		// If the color is in the wrong format, the user either is in an unsupported browser or has manipulated the
		// input field.
		let Ok(color) = color_from_rgb_str(&color_signal.get()) else { return; };

		let registration_data = UserRegistrationFinalize {
			name: (*username).clone(),
			color,
		};

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = UserRegistration::Finalize(registration_data);
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						String::from(
							"Failed to serialize registration message. Ensure your username is valid and try again.",
						),
						error,
					)));
					navigate("/error");
					return;
				}
			};

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("Failed to send registration message"),
					error,
				)));
				navigate("/error");
				return;
			}

			let response: DataMessage<RegistrationResponse> = match read_websocket(&mut ws).await {
				Ok(data) => data,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						String::from("Failed to receive registration response"),
						error,
					)));
					navigate("/error");
					return;
				}
			};

			let response = match response {
				Ok(resp) => resp,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						String::from("A server error occurred during registration."),
						error,
					)));
					navigate("/error");
					return;
				}
			};

			match response {
				RegistrationResponse::Success(user) => {
					let user_data_signal: &Signal<Option<UserData>> = use_context(ctx);
					user_data_signal.set(Some(user));
					navigate("/register_complete");
				}
				RegistrationResponse::UsernameInUse => username_in_use_signal.set(true),
				_ => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new(String::from("A desync occurred in validation expectations between the client and the server. Please refresh the page."))));
					navigate("/error");
				}
			}
		});
	};

	create_effect(ctx, move || {
		let username = (*username_signal.get()).clone();

		if username.is_empty() {
			username_in_use_signal.set(false);
			return;
		}

		if username.len() > USERNAME_LENGTH_LIMIT {
			username_in_use_signal.set(false);
			return;
		}

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = UserRegistration::CheckUsername(username);
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(String::from("Failed to serialize username availability check. Ensure your username is valid and try again."), error)));
					navigate("/error");
					return;
				}
			};

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("Failed to send username availability check message."),
					error,
				)));
				navigate("/error");
				return;
			}

			let response: DataMessage<UsernameCheckResponse> = match read_websocket(&mut ws).await {
				Ok(data) => data,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						String::from("Failed to receive username availability check response."),
						error,
					)));
					navigate("/error");
					return;
				}
			};

			let response = match response {
				Ok(resp) => resp,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						String::from("A server error occurred checking username availability."),
						error,
					)));
					navigate("/error");
					return;
				}
			};

			if *username_signal.get() == response.username {
				match response.status {
					UsernameCheckStatus::Available => username_in_use_signal.set(false),
					UsernameCheckStatus::Unavailable => username_in_use_signal.set(true),
				}
			}
		});
	});

	view! {
		ctx,
		h1 { "Register an Account" }
		form(id="register_user", on:submit=form_submission_handler) {
			div(class="input_with_message") {
				label(for="register_username") {
					"Username: "
				}
				input(id="register_username", type="text", class=*username_error_class_signal.get(), bind:value=username_signal, ref=username_field)
				(
					if *username_in_use_signal.get() {
						view! {
							ctx,
							span(id="register_username_in_use_warning", class="input_error register_username_error") {
								"This username is in use."
							}
						}
					} else {
						view! { ctx, }
					}
				)
				(
					if *username_empty_signal.get() {
						view! {
							ctx,
							span(id="register_username_empty_warning", class="input_error register_username_error") {
								"Username cannot be empty."
							}
						}
					} else {
						view! { ctx, }
					}
				)
				(
					if *username_too_long_signal.get() {
						view! {
							ctx,
							span(id="register_username_too_long_warning", class="input_error register_username_error") {
								"Username is too long."
							}
						}
					} else {
						view! { ctx, }
					}
				)
			}
			ColorInputWithContrast(color=color_signal, username=username_signal, view_id="register_user")
			div(id="register_contrast_help_notice") { "For best readability, it's recommended to choose a color with contrast values of at least 4.5." }
			button(ref=submit_button_ref) {
				"Register"
			}
		}
	}
}
