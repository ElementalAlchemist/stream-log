// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use async_std::stream::StreamExt;
use tide_websockets::{Message, WebSocketConnection};

pub enum WebSocketRecvError {
	StreamExhausted,
	WebSocketError(tide_websockets::Error),
	BinaryMessage,
	ConnectionClosed,
}

impl WebSocketRecvError {
	/// Logs the WebSocket receive error to the current tide connection. Intended to be
	/// called prior to ending the handling of the connection.
	pub fn log(&self) {
		match self {
			Self::StreamExhausted => tide::log::error!("The WebSocket connection stream was exhausted."),
			Self::WebSocketError(error) => {
				tide::log::error!("An error occurred with the WebSocket connection: {}", error)
			}
			Self::BinaryMessage => tide::log::error!("A binary message was received on the connection"),
			Self::ConnectionClosed => tide::log::info!("The WebSocket connection was closed by the client"),
		}
	}
}

impl From<tide_websockets::Error> for WebSocketRecvError {
	fn from(error: tide_websockets::Error) -> Self {
		Self::WebSocketError(error)
	}
}

/// Receives a single message from the connection. Handles details related to receiving the data,
/// including errors, connection closures, and ping/pong messages.
pub async fn recv_msg(stream: &mut WebSocketConnection) -> Result<String, WebSocketRecvError> {
	loop {
		let message_data = stream.next().await;
		let message_data = match message_data {
			Some(data) => data,
			None => break Err(WebSocketRecvError::StreamExhausted),
		}?;
		match message_data {
			Message::Binary(_) => break Err(WebSocketRecvError::BinaryMessage),
			Message::Text(text) => break Ok(text),
			Message::Ping(data) => {
				stream.send(Message::Pong(data)).await?;
			}
			Message::Pong(_) => (),
			Message::Close(_) => break Err(WebSocketRecvError::ConnectionClosed),
		}
	}
}
