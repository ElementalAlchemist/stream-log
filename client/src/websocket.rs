use futures::stream::{SplitSink, SplitStream};
use futures::StreamExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::{Message, WebSocketError};
use serde::de::DeserializeOwned;
use std::fmt::Display;
use web_sys::Url;

/// Errors that can occur when reading data from a WebSocket connection
pub enum WebSocketReadError {
	ConnectionClosed,
	BinaryMessage,
	WebSocketError(WebSocketError),
	JsonError(serde_json::Error),
}

impl Display for WebSocketReadError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::JsonError(err) => write!(f, "{}", err),
			Self::WebSocketError(err) => write!(f, "{}", err),
			Self::ConnectionClosed => write!(f, "WebSocket connection closed"),
			Self::BinaryMessage => write!(f, "An unexpected binary message was received from the WebSocket"),
		}
	}
}

impl From<serde_json::Error> for WebSocketReadError {
	fn from(error: serde_json::Error) -> Self {
		Self::JsonError(error)
	}
}

impl From<WebSocketError> for WebSocketReadError {
	fn from(error: WebSocketError) -> Self {
		Self::WebSocketError(error)
	}
}

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

/// Reads a single unit of data from a WebSocket connection.
///
/// # Errors
///
/// Errors occur in a variety of situations: when the connection unexpectedly closes,
/// when we unexpectedly get binary data, when there's an error reading from the connection,
/// and when the text can't be deserialized appropriately as JSON.
pub async fn read_websocket<T: DeserializeOwned>(
	read_stream: &mut SplitStream<WebSocket>,
) -> Result<T, WebSocketReadError> {
	let msg = match read_stream.next().await {
		Some(data) => data?,
		None => return Err(WebSocketReadError::ConnectionClosed),
	};
	let msg = match msg {
		Message::Text(text) => text,
		Message::Bytes(_) => return Err(WebSocketReadError::BinaryMessage),
	};
	Ok(serde_json::from_str(&msg)?)
}

pub struct WebSocketHandle {
	write: SplitSink<WebSocket, Message>,
	read: SplitStream<WebSocket>,
}
