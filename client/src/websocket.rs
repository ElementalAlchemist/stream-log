// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::{Message, WebSocketError};
use serde::de::DeserializeOwned;
use std::collections::VecDeque;
use std::fmt::Display;
use wasm_bindgen::JsCast;
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
	let doc = web_sys::window()
		.expect("Failed to get browser window context")
		.document()
		.expect("Failed to get webpage document root");
	let doc_node: web_sys::Node = doc.unchecked_into();
	let web_endpoint = doc_node
		.base_uri()
		.expect("Failed to get base address")
		.expect("Failed to get base address");
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
	let Message::Text(msg) = msg else {
		return Err(WebSocketReadError::BinaryMessage);
	};
	Ok(serde_json::from_str(&msg)?)
}

/// Represents the send handle for a WebSocket. Handles temporary disconnections.
pub struct WebSocketSendStream {
	write_stream: Option<SplitSink<WebSocket, Message>>,
	disconnected_message_queue: VecDeque<Message>,
}

impl WebSocketSendStream {
	pub fn new(write_stream: SplitSink<WebSocket, Message>) -> Self {
		let write_stream = Some(write_stream);
		let disconnected_message_queue = VecDeque::new();
		Self {
			write_stream,
			disconnected_message_queue,
		}
	}

	pub async fn send(&mut self, message: Message) -> Result<(), WebSocketError> {
		match &mut self.write_stream {
			Some(stream) => stream.send(message).await,
			None => {
				self.disconnected_message_queue.push_back(message);
				Ok(())
			}
		}
	}

	pub async fn send_multiple(&mut self, messages: impl IntoIterator<Item = Message>) -> Result<(), WebSocketError> {
		match &mut self.write_stream {
			Some(stream) => {
				for message in messages {
					stream.feed(message).await?;
				}
				stream.flush().await
			}
			None => {
				for message in messages {
					self.disconnected_message_queue.push_back(message);
				}
				Ok(())
			}
		}
	}

	pub fn mark_disconnected(&mut self) {
		self.write_stream = None;
	}

	pub fn set_new_connection(&mut self, write_stream: SplitSink<WebSocket, Message>) {
		self.write_stream = Some(write_stream);
	}
}
