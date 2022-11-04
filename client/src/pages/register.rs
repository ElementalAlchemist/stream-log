use super::error::ErrorData;
use crate::websocket::read_websocket;
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::{SinkExt, StreamExt};
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

enum PageTask {
	CheckUsername(String),
	SubmitRegistration(UserRegistrationFinalize),
}

#[component]
pub async fn RegistrationView<G: Html>(ctx: Scope<'_>) -> View<G> {
	log::debug!("Activating registration page");

	let username_signal = create_signal(ctx, String::new());
	let username_in_use_signal = create_signal(ctx, false);
	let username_empty_signal = create_signal(ctx, false); // This shouldn't be visible until some user entry happens
	let username_too_long_signal = create_signal(ctx, false);
	let username_field = create_node_ref(ctx);
	let submit_button_ref = create_node_ref(ctx);

	// Username error signal collects all possible kinds of username errors
	let username_error_signal = create_memo(ctx, || {
		*username_in_use_signal.get() || *username_empty_signal.get() || *username_too_long_signal.get()
	});
	// Username error class signal determines what the class of the username field should be based on whether there's an error
	let username_error_class_signal = create_memo(ctx, || if *username_error_signal.get() { "error" } else { "" });

	let (form_tx, mut form_rx) = mpsc::unbounded();

	let form_submission_handler = {
		let form_tx = form_tx.clone();
		move |_| {
			let username = username_signal.get();
			if username.is_empty() {
				username_empty_signal.set(true);
				return;
			}
			let registration_data = UserRegistrationFinalize {
				name: (*username).clone(),
			};
			if let Err(error) = form_tx.unbounded_send(PageTask::SubmitRegistration(registration_data)) {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("An internal communication error occurred"),
					error,
				)));
				navigate("/error");
			}
		}
	};

	create_effect(ctx, move || {
		let new_username = (*username_signal.get()).clone();
		// The username was modified, so clear submit-only errors
		username_empty_signal.set(new_username.is_empty());
		username_too_long_signal.set(new_username.len() > USERNAME_LENGTH_LIMIT);

		if let Err(error) = form_tx.unbounded_send(PageTask::CheckUsername(new_username)) {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("An internal communication error occurred"),
				error,
			)));
			navigate("/error");
		}
	});

	spawn_local_scoped(ctx, async move {
		while let Some(task) = form_rx.next().await {
			match task {
				PageTask::CheckUsername(username) => {
					if username.is_empty() {
						username_empty_signal.set(true);
						username_in_use_signal.set(false);
						continue;
					}
					username_empty_signal.set(false);

					let ws_context: &Mutex<WebSocket> = use_context(ctx);
					let mut ws = ws_context.lock().await;

					let message = UserRegistration::CheckUsername(username);
					let message_json = match serde_json::to_string(&message) {
						Ok(msg) => msg,
						Err(error) => {
							let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
							error_signal.set(Some(ErrorData::new_with_error(String::from("Failed to serialize username availability check. Ensure your username is valid and try again."), error)));
							navigate("/error");
							break;
						}
					};
					if let Err(error) = ws.send(Message::Text(message_json)).await {
						let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
						error_signal.set(Some(ErrorData::new_with_error(
							String::from("Failed to send username availability check message."),
							error,
						)));
						navigate("/error");
						break;
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
							break;
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
							break;
						}
					};
					if *username_signal.get() == response.username {
						match response.status {
							UsernameCheckStatus::Available => username_in_use_signal.set(false),
							UsernameCheckStatus::Unavailable => username_in_use_signal.set(true),
						}
					}
				}
				PageTask::SubmitRegistration(registration_data) => {
					let ws_context: &Mutex<WebSocket> = use_context(ctx);
					let mut ws = ws_context.lock().await;

					let message = UserRegistration::Finalize(registration_data);
					let message_json = match serde_json::to_string(&message) {
						Ok(msg) => msg,
						Err(error) => {
							let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
							error_signal.set(Some(ErrorData::new_with_error(String::from("Failed to serialize registration message. Ensure your username is valid and try again."), error)));
							navigate("/error");
							break;
						}
					};
					if let Err(error) = ws.send(Message::Text(message_json)).await {
						let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
						error_signal.set(Some(ErrorData::new_with_error(
							String::from("Failed to send registration message"),
							error,
						)));
						navigate("/error");
						break;
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
							break;
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
							break;
						}
					};
					match response {
						RegistrationResponse::Success(user) => {
							let user_data_signal: &Signal<Option<UserData>> = use_context(ctx);
							user_data_signal.set(Some(user));
							navigate("/register_complete");
						}
						RegistrationResponse::NoUsernameSpecified => username_empty_signal.set(true),
						RegistrationResponse::UsernameInUse => username_in_use_signal.set(true),
						RegistrationResponse::UsernameTooLong => username_too_long_signal.set(true),
					}
				}
			}
		}
	});

	view! {
		ctx,
		form(id="register_user", on:submit=form_submission_handler) {
			label(for="register_username") {
				"Username: "
			}
			input(id="register_username", type="text", class=*username_error_class_signal.get(), bind:value=username_signal, ref=username_field)
			(
				if *username_in_use_signal.get() {
					view! {
						ctx,
						div(id="register_username_in_use_warning", class="register_username_error") {
							"This username is in use."
						}
					}
				} else if *username_empty_signal.get() {
					view! {
						ctx,
						div(id="register_username_empty_warning", class="register_username_error") {
							"Username cannot be empty."
						}
					}
				} else if *username_too_long_signal.get() {
					view! {
						ctx,
						div(id="register_username_too_long_warning", class="register_username_error") {
							"Username is too long."
						}
					}
				} else {
					view! { ctx, }
				}
			)
			button(ref=submit_button_ref) {
				"Register"
			}
		}
	}
}
