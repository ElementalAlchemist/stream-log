use crate::color_utils::color_from_rgb_str;
use crate::components::color_input_with_contrast::ColorInputWithContrast;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::user_register::{
	RegistrationFinalizeResponse, UserRegistration, UserRegistrationFinalize, USERNAME_LENGTH_LIMIT,
};
use stream_log_shared::messages::FromClientMessage;
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

	create_effect(ctx, || {
		let data: &DataSignals = use_context(ctx);
		let registration_data = data.registration.final_register.get();
		if let Some(reg_data) = registration_data.as_ref() {
			match reg_data {
				RegistrationFinalizeResponse::Success(data) => {
					let user_signal: &Signal<Option<UserData>> = use_context(ctx);
					user_signal.set(Some(data.clone()));
					navigate("/register_complete");
				}
				RegistrationFinalizeResponse::NoUsernameSpecified => data.errors.modify().push(ErrorData::new(
					"You didn't enter a username. Enter your registration data and try again.",
				)),
				RegistrationFinalizeResponse::UsernameInUse => data.errors.modify().push(ErrorData::new(
					"The username you entered is already in use. Select another one and try again.",
				)),
				RegistrationFinalizeResponse::UsernameTooLong => data.errors.modify().push(ErrorData::new(
					"The username you entered is too long. Select a shorter name and try again.",
				)),
			}
		}
	});

	let data: &DataSignals = use_context(ctx);

	let username_signal = create_signal(ctx, String::new());
	let username_in_use_signal = create_memo(ctx, || {
		username_signal.track();
		if let Some(check_data) = data.registration.username_check.get().as_ref() {
			check_data.username == *username_signal.get() && check_data.available
		} else {
			false
		}
	});
	let username_empty_signal = create_memo(ctx, || username_signal.get().is_empty());
	let username_too_long_signal = create_memo(ctx, || username_signal.get().len() > USERNAME_LENGTH_LIMIT);
	let username_field = create_node_ref(ctx);
	let color_signal = create_signal(ctx, String::from("#7f7f7f"));
	let submit_button_ref = create_node_ref(ctx);

	// Username error signal collects all possible kinds of username errors
	let username_error_signal = create_memo(ctx, || {
		*username_in_use_signal.get() || *username_empty_signal.get() || *username_too_long_signal.get()
	});

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
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::RegistrationRequest(UserRegistration::Finalize(registration_data));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize registration message. Ensure your username is valid and try again.",
						error,
					));
					return;
				}
			};

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send registration message.", error));
				return;
			}
		});
	};

	create_effect(ctx, move || {
		let username = (*username_signal.get()).clone();
		let data: &DataSignals = use_context(ctx);

		if username.is_empty() {
			data.registration.username_check.set(None);
			return;
		}

		if username.len() > USERNAME_LENGTH_LIMIT {
			data.registration.username_check.set(None);
			return;
		}

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::RegistrationRequest(UserRegistration::CheckUsername(username));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize username availability check. Ensure your username is valid and try again.",
						error,
					));
					return;
				}
			};

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to send username availability check message.",
					error,
				));
				return;
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
				input(id="register_username", type="text", class=if *username_error_signal.get() { "error" } else { "" }, bind:value=username_signal, ref=username_field)
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
