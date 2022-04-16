use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use mogwai::prelude::*;
use std::panic;
use stream_log_shared::messages::initial::InitialMessage;

mod websocket;
use websocket::websocket_endpoint;

fn main() {
	panic::set_hook(Box::new(console_error_panic_hook::hook));
	
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
		let output = match msg_data {
			InitialMessage::Welcome => "Welcome! You got mail!",
			InitialMessage::Unauthorized(_) => "Unauthorized access. Access denied."
		};
		let _ = ws_write.close().await;
		let view_builder: ViewBuilder<Dom> = builder! {
			<div>
				{output}
			</div>
		};
		let view: View<Dom> = view_builder.try_into().expect("Failed to convert view to DOM");
		view.run().expect("Failed to host view");
	});
}
