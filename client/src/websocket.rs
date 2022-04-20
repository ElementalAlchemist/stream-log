use web_sys::Url;

/// Gets the URL of the websocket endpoint in a way that adapts to any URL structure at which the application could be
/// hosted.
///
/// # Panics
///
/// This function panics when the browser context (window, location, URL, etc.) is inaccessible.
pub fn websocket_endpoint() -> String {
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
