use futures::stream::{SplitSink, SplitStream};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::{Message, WebSocketError};
use mogwai::prelude::*;
use std::fmt::Display;
use stream_log_shared::messages::user::{UserRegistration, UserRegistrationFinalize};
use web_sys::FormData;

pub enum RegistrationError {
	JsonError(serde_json::Error),
	WebSocketError(WebSocketError)
}

impl Display for RegistrationError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::JsonError(err) => write!(f, "{}", err),
			Self::WebSocketError(err) => write!(f, "{}", err)
		}
	}
}

impl From<serde_json::Error> for RegistrationError {
	fn from(error: serde_json::Error) -> Self {
		Self::JsonError(error)
	}
}

impl From<WebSocketError> for RegistrationError {
	fn from(error: WebSocketError) -> Self {
		Self::WebSocketError(error)
	}
}

pub async fn run_page(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
) -> Result<(), RegistrationError> {
	let (form_tx, mut form_rx) = broadcast::bounded(1);
	let page_view: View<Dom> = view! {
		<form id="registration" on:submit=form_tx.sink().contra_map(|dom_event: DomEvent| {
			let browser_event = dom_event.browser_event().unwrap();
			browser_event.prevent_default();
			FormData::new_with_form(browser_event.current_target().unwrap().dyn_ref().unwrap())
		})>
			<h1>"New User Registration"</h1>
			<div>
				<label for="username">"Username:"</label>
				<input type="text" id="username" name="username" />
			</div>
			<div>
				<button type="submit">"Create User"</button>
			</div>
		</form>
	};
	page_view.run().expect("Failed to host registration page");
	let form_data = form_rx.next().await.unwrap().unwrap();
	let name = form_data.get("username").as_string().unwrap();
	let user_data = UserRegistrationFinalize { name };
	let user_data = UserRegistration::Finalize(user_data);
	let user_data_json = serde_json::to_string(&user_data)?;
	Ok(ws_write.send(Message::Text(user_data_json)).await?)
}
