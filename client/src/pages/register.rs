use crate::websocket::{read_websocket, WebSocketReadError};
use futures::stream::{SplitSink, SplitStream};
use futures::{join, select};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use mogwai::prelude::*;
use mogwai::utils::document;
use stream_log_shared::messages::user_register::{
	RegistrationResponse, UserRegistration, UserRegistrationFinalize, UsernameCheckResponse, UsernameCheckStatus,
};
use stream_log_shared::messages::DataError;
use web_sys::{FormData, HtmlInputElement};

const MAX_USERNAME_LEN: u32 = 64;
const USERNAME_AVAILABLE_DESC: &str = "This username is available.";
const USERNAME_UNAVAILABLE_DESC: &str = "This username is not available.";
const USERNAME_LONG_DESC: &str = "The entered username is too long.";
const USERNAME_AVAILABLE_CLASS: &str = "username-available";
const USERNAME_UNAVAILABLE_CLASS: &str = "username-unavailable";
const SEND_CHANNEL_ERROR_MSG: &str = "A DOM control channel for registration closed unexpectedly.";

/// Types of errors that can occur to prevent successful registration.
pub enum RegistrationError {
	WebSocketReadError(WebSocketReadError),
	ServerDataError(DataError),
}

impl From<DataError> for RegistrationError {
	fn from(error: DataError) -> Self {
		Self::ServerDataError(error)
	}
}

impl From<WebSocketReadError> for RegistrationError {
	fn from(error: WebSocketReadError) -> Self {
		Self::WebSocketReadError(error)
	}
}

pub async fn run_page(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
) -> Result<(), WebSocketReadError> {
	let (form_tx, mut form_rx) = broadcast::bounded(1);
	let (username_change_tx, mut username_change_rx) = broadcast::bounded(1);
	let (username_class_tx, username_class_rx) = broadcast::bounded(1);
	let (username_avail_desc_tx, username_avail_desc_rx) = broadcast::bounded(1);
	let page_view: View<Dom> = view! {
		<form id="registration" on:submit=form_tx.sink().contra_map(|dom_event: DomEvent| {
			let browser_event = dom_event.browser_event().unwrap();
			browser_event.prevent_default();
			FormData::new_with_form(browser_event.current_target().unwrap().dyn_ref().unwrap()).unwrap()
		})>
			<h1>"New User Registration"</h1>
			<div>
				<label for="username">"Username:"</label>
				<input
					type="text"
					id="username"
					name="username"
					maxlength=MAX_USERNAME_LEN.to_string()
					class=("", username_class_rx.clone())
					on:change=username_change_tx.sink().contra_map(
						|event: DomEvent|
							event
								.browser_event()
								.unwrap()
								.current_target()
								.unwrap()
								.dyn_ref::<HtmlInputElement>()
								.unwrap()
								.value()
					)
				/>
				<span class=("", username_class_rx)>{("", username_avail_desc_rx)}</span>
			</div>
			<div>
				<button type="submit">"Create User"</button>
			</div>
		</form>
	};
	page_view.run().expect("Failed to host registration page");
	let mut form_future = form_rx.next();
	let mut username_check_future = username_change_rx.next();
	loop {
		select! {
			username = username_check_future => {
				let username = if let Some(name) = username { name } else { continue; };
				if username.is_empty() {
					let class_send = username_class_tx.broadcast(String::new());
					let avail_msg_send = username_avail_desc_tx.broadcast(String::new());
					let (class_res, avail_msg_res) = join!(class_send, avail_msg_send);
					class_res.expect(SEND_CHANNEL_ERROR_MSG);
					avail_msg_res.expect(SEND_CHANNEL_ERROR_MSG);
					username_check_future = username_change_rx.next();
					continue;
				}
				let user_check = UserRegistration::CheckUsername(username.clone());
				let user_check_json = serde_json::to_string(&user_check)?;
				ws_write.send(Message::Text(user_check_json)).await?;
				let response: UsernameCheckResponse = read_websocket(ws_read).await?;
				let current_username = document().get_element_by_id("username").unwrap().dyn_ref::<HtmlInputElement>().unwrap().value();
				if username != current_username {
					let class_send = username_class_tx.broadcast(String::new());
					let avail_msg_send = username_avail_desc_tx.broadcast(String::new());
					let (class_res, avail_msg_res) = join!(class_send, avail_msg_send);
					class_res.expect(SEND_CHANNEL_ERROR_MSG);
					avail_msg_res.expect(SEND_CHANNEL_ERROR_MSG);
					username_check_future = username_change_rx.next();
					continue;
				}
				let (class_send, avail_msg_send) = match response.status {
					UsernameCheckStatus::Available => (
						username_class_tx.broadcast(String::from(USERNAME_AVAILABLE_CLASS)),
						username_avail_desc_tx.broadcast(String::from(USERNAME_AVAILABLE_DESC))
					),
					UsernameCheckStatus::Unavailable => (
						username_class_tx.broadcast(String::from(USERNAME_UNAVAILABLE_CLASS)),
						username_avail_desc_tx.broadcast(String::from(USERNAME_UNAVAILABLE_DESC))
					)
				};
				let (class_res, avail_msg_res) = join!(class_send, avail_msg_send);
				class_res.expect(SEND_CHANNEL_ERROR_MSG);
				avail_msg_res.expect(SEND_CHANNEL_ERROR_MSG);
				username_check_future = username_change_rx.next();
			}
			form_data = form_future => {
				let form_data = if let Some(data) = form_data { data } else { continue; };
				let username = form_data.get("username").as_string().unwrap();
				let final_data = UserRegistrationFinalize { name: username.clone() };
				let registration = UserRegistration::Finalize(final_data);
				let registration_json = serde_json::to_string(&registration)?;
				ws_write.send(Message::Text(registration_json)).await?;
				let response: RegistrationResponse = read_websocket(ws_read).await?;
				match response {
					RegistrationResponse::Success => break,
					RegistrationResponse::UsernameInUse => {
						let class_send = username_class_tx.broadcast(String::from(USERNAME_UNAVAILABLE_CLASS));
						let avail_msg_send = username_avail_desc_tx.broadcast(String::from(USERNAME_UNAVAILABLE_DESC));
						let (class_res, avail_msg_res) = join!(class_send, avail_msg_send);
						class_res.expect(SEND_CHANNEL_ERROR_MSG);
						avail_msg_res.expect(SEND_CHANNEL_ERROR_MSG);
						form_future = form_rx.next();
					}
					RegistrationResponse::UsernameTooLong => {
						let class_send = username_class_tx.broadcast(String::from(USERNAME_UNAVAILABLE_CLASS));
						let avail_msg_send = username_avail_desc_tx.broadcast(String::from(USERNAME_LONG_DESC));
						let (class_res, avail_msg_res) = join!(class_send, avail_msg_send);
						class_res.expect(SEND_CHANNEL_ERROR_MSG);
						avail_msg_res.expect(SEND_CHANNEL_ERROR_MSG);
						form_future = form_rx.next();
					}
				}
			}
		}
	}
	Ok(())
}
