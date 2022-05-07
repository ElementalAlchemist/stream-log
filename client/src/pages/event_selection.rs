use crate::websocket::{WebSocketReadError, read_websocket};
use futures::stream::{SplitSink, SplitStream};
use mogwai::prelude::*;
use std::fmt;
use gloo_net::websocket::{Message, WebSocketError};
use gloo_net::websocket::futures::WebSocket;
use stream_log_shared::messages::{DataError, DataMessage};
use stream_log_shared::messages::events::{Event, EventSelection};

pub enum EventSelectionError {
	WebSocketReadError(WebSocketReadError),
	WebSocketSendError(WebSocketError),
	ServerDataError(DataError),
	MessageTypeError(serde_json::Error),
	ChannelSendError(mogwai::channel::mpsc::SendError)
}

impl From<WebSocketReadError> for EventSelectionError {
	fn from(error: WebSocketReadError) -> Self {
		Self::WebSocketReadError(error)
	}
}

impl From<WebSocketError> for EventSelectionError {
	fn from(error: WebSocketError) -> Self {
		Self::WebSocketSendError(error)
	}
}

impl From<DataError> for EventSelectionError {
	fn from(error: DataError) -> Self {
		Self::ServerDataError(error)
	}
}

impl From<serde_json::Error> for EventSelectionError {
	fn from(error: serde_json::Error) -> Self {
		Self::MessageTypeError(error)
	}
}

impl From<mogwai::channel::mpsc::SendError> for EventSelectionError {
	fn from(error: mogwai::channel::mpsc::SendError) -> Self {
		Self::ChannelSendError(error)
	}
}

impl fmt::Display for EventSelectionError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::WebSocketReadError(error) => write!(f, "Failed to read from WebSocket: {}", error),
			Self::WebSocketSendError(error) => write!(f, "Failed to send over WebSocket: {}", error),
			Self::ServerDataError(error) => write!(f, "The server encountered an error: {}", error),
			Self::MessageTypeError(error) => write!(f, "An invalid message was received: {}", error),
			Self::ChannelSendError(error) => write!(f, "An internal communication error occurred: {}", error)
		}
	}
}

pub async fn run_page(ws_write: &mut SplitSink<WebSocket, Message>, ws_read: &mut SplitStream<WebSocket>) -> Result<Event, EventSelectionError> {
	let (mut event_tx, event_rx) = mogwai::channel::mpsc::channel(1);
	let (select_tx, mut select_rx) = mogwai::channel::broadcast::bounded(1);
	let event_view = view! {
		<div id="event_selector">
			<h1>"Event Selection"</h1>
			<ul patch:children=event_rx></ul>
		</div>
	};
	event_view.run().expect("Failed to host event selection");
	let event_selection: DataMessage<EventSelection> = read_websocket(ws_read).await?;
	let event_selection = event_selection?;
	for event in event_selection.available_events.iter() {
		let event_builder = builder! {
			<li
				data-event-id=event.id.clone()
				class="event-click"
				on:click=select_tx.sink().contra_map({
					let event = event.clone();
					move |_: DomEvent| {
						event.clone()
					}
				})
			>
				{event.name.clone()}
			</li>
		};
		event_tx.send(ListPatch::push(event_builder)).await?;
	}
	let selection: Event = select_rx.next().await.unwrap();
	let selection_json = serde_json::to_string(&selection)?;
	ws_write.send(Message::Text(selection_json)).await?;
	Ok(selection)
}