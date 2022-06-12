use super::dom::run_view;
use super::user_info_bar::{UserBarBuildData, UserClickTarget};
use super::websocket::WebSocketReadError;
use gloo_net::websocket::WebSocketError;
use mogwai::prelude::*;
use std::fmt;
use stream_log_shared::messages::DataError;

pub enum PageError {
	WebSocketRead(WebSocketReadError),
	WebSocketSend(WebSocketError),
	ServerData(DataError),
	MessageType(serde_json::Error),
	ChannelSend(mogwai::channel::mpsc::SendError),
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

impl From<mogwai::channel::mpsc::SendError> for PageError {
	fn from(error: mogwai::channel::mpsc::SendError) -> Self {
		Self::ChannelSend(error)
	}
}

impl fmt::Display for PageError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::WebSocketRead(error) => write!(f, "Failed to read from WebSocket: {}", error),
			Self::WebSocketSend(error) => write!(f, "Failed to send over WebSocket: {}", error),
			Self::ServerData(error) => write!(f, "The server encountered an error: {}", error),
			Self::MessageType(error) => write!(f, "An invalid message was received: {}", error),
			Self::ChannelSend(error) => write!(f, "An internal communication error occurred: {}", error),
		}
	}
}

pub fn render_error_message(message: &str) {
	let error_view = builder! {
		<div class="error">
			{message}
		</div>
	};
	let user_bar_build_data: Option<UserBarBuildData<UserClickTarget>> = None;
	run_view(error_view, user_bar_build_data).expect("Failed to host view");
}

pub fn render_error_message_with_details(message: &str, error: impl fmt::Display) {
	let error_view = builder! {
		<div class="error">
			{message}
			<br />
			{error.to_string()}
		</div>
	};
	let user_bar_build_data: Option<UserBarBuildData<UserClickTarget>> = None;
	run_view(error_view, user_bar_build_data).expect("Failed to host view");
}
