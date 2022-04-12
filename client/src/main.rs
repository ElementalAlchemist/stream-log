use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use mogwai::prelude::*;
use stream_log_shared::messages::initial::InitialMessage;
use web_sys::Url;

mod js_def;

/// Gets the URL of the websocket endpoint in a way that adapts to any URL structure at which the application could be
/// hosted.
///
/// # Panics
///
/// This function panics when the browser context (window, location, URL, etc.) is inaccessible.
fn websocket_endpoint() -> String {
	let js_location = web_sys::window()
		.expect("Failed to get browser window context")
		.location();
	let web_endpoint = js_location.href().expect("Failed to get current address");
	let url = Url::new(&web_endpoint).expect("Failed to generate URL instance");
	url.set_search(""); // Query string is unnecessary and should be cleared
	if url.protocol() == "http:" {
		url.set_protocol("ws:");
	} else {
		url.set_protocol("wss:");
	}
	let url_path = url.pathname();
	let ws_path = if let Some(path) = url_path.strip_suffix('/') {
		format!("{}/ws", path)
	} else {
		format!("{}/ws", url_path)
	};
	url.set_pathname(&ws_path);
	url.to_string().into()
}

fn main() {
	mogwai::spawn(async {
		let ws = match WebSocket::open(websocket_endpoint().as_str()) {
			Ok(ws) => ws,
			Err(error) => {
				let error_builder: ViewBuilder<Dom> = builder! {
					<div class="error">
						"Unable to load/operate: A websocket connection could not be formed."
						<br />
						{error.to_string()}
					</div>
				};
				let error_view: View<Dom> = error_builder.try_into().expect("Failed to convert WS error to DOM");
				error_view.run().expect("Failed to host view");
				return;
			}
		};
		let (mut ws_write, mut ws_read) = ws.split();
		let msg = match ws_read.next().await {
			Some(Ok(msg)) => msg,
			Some(Err(error)) => {
				let error_builder: ViewBuilder<Dom> = builder! {
					<div class="error">
						"Unable to load/operate: Failed to receive initial websocket message"
						<br />
						{error.to_string()}
					</div>
				};
				let error_view: View<Dom> = error_builder.try_into().expect("Failed to convert recv error to DOM");
				error_view.run().expect("Failed to host view");
				return;
			}
			None => {
				let error_builder: ViewBuilder<Dom> = builder! {
					<div class="error">
						"Unable to load/operate: Failed to receive initial websocket message (connection closed without content)"
					</div>
				};
				let error_view: View<Dom> = error_builder
					.try_into()
					.expect("Failed to convert WS recv error to DOM");
				error_view.run().expect("Failed to host view");
				return;
			}
		};
		let msg = match msg {
			Message::Text(txt) => txt,
			Message::Bytes(_) => unimplemented!(),
		};
		let msg_data: InitialMessage = serde_json::from_str(&msg).expect("Message data was of the incorrect type");
		match msg_data {
			InitialMessage::Welcome => todo!(),
			InitialMessage::Unauthorized(unauth_data) => {
				js_def::launch_google_auth_flow(unauth_data.google_auth_client_id());
			}
		}
		ws_write.close().await;
	});
}
