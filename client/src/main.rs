use mogwai::prelude::*;
use gloo_net::websocket::futures::WebSocket;
use web_sys::Url;

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
		let msg = ws_read.next().await;
		let response_builder: ViewBuilder<Dom> = builder! {
			<div class="response">
				"Response received:"
				<br />
				{format!("{:?}", msg)}
			</div>
		};
		let response_view: View<Dom> = response_builder.try_into().expect("Failed to create view for response");
		response_view.run().expect("Failed to host response view");
	});
}
