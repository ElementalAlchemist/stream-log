use super::websocket::WebSocketReadError;
use gloo_net::websocket::WebSocketError;
use std::fmt;
use stream_log_shared::messages::DataError;

pub enum PageError {
	WebSocketRead(WebSocketReadError),
	WebSocketSend(WebSocketError),
	ServerData(DataError),
	MessageType(serde_json::Error),
}

impl From<WebSocketReadError> for PageError {
	fn from(error: WebSocketReadError) -> Self {
		Self::WebSocketRead(error)
	}
}

impl From<WebSocketError> for PageError {
	fn from(error: WebSocketError) -> Self {
		Self::WebSocketSend(error)
	}
}

impl From<DataError> for PageError {
	fn from(error: DataError) -> Self {
		Self::ServerData(error)
	}
}

impl From<serde_json::Error> for PageError {
	fn from(error: serde_json::Error) -> Self {
		Self::MessageType(error)
	}
}

impl fmt::Display for PageError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::WebSocketRead(error) => write!(f, "Failed to read from WebSocket: {}", error),
			Self::WebSocketSend(error) => write!(f, "Failed to send over WebSocket: {}", error),
			Self::ServerData(error) => write!(f, "The server encountered an error: {}", error),
			Self::MessageType(error) => write!(f, "An invalid message was received: {}", error),
		}
	}
}
