use mogwai::prelude::*;
use web_sys::Url;
use ws_stream_wasm::WsMeta;

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
	url.set_protocol("wss:");
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
		let (conn_metadata, conn_stream) = match WsMeta::connect(websocket_endpoint(), None).await {
			Ok(conn_data) => conn_data,
			Err(error) => {
				let error_builder: ViewBuilder<Dom> = builder! {
					<div class="error">
						"Unable to load/function: A websocket connection could not be formed."
					</div>
				};
				let error_view: View<Dom> = error_builder.try_into().expect("Failed to convert WS error to DOM");
				error_view.run().expect("Failed to host view");
				return;
			}
		};
	});
}
