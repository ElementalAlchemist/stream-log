use super::admin;
use crate::dom::run_view;
use crate::user_info_bar::ClickTarget;
use crate::websocket::{read_websocket, WebSocketReadError};
use futures::select;
use futures::stream::{SplitSink, SplitStream};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::{Message, WebSocketError};
use mogwai::prelude::*;
use std::fmt;
use stream_log_shared::messages::events::{Event, EventSelection};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataError, DataMessage, PageControl};

pub enum EventSelectionError {
	WebSocketRead(WebSocketReadError),
	WebSocketSend(WebSocketError),
	ServerData(DataError),
	MessageType(serde_json::Error),
	ChannelSend(mogwai::channel::mpsc::SendError),
}

impl From<WebSocketReadError> for EventSelectionError {
	fn from(error: WebSocketReadError) -> Self {
		Self::WebSocketRead(error)
	}
}

impl From<WebSocketError> for EventSelectionError {
	fn from(error: WebSocketError) -> Self {
		Self::WebSocketSend(error)
	}
}

impl From<DataError> for EventSelectionError {
	fn from(error: DataError) -> Self {
		Self::ServerData(error)
	}
}

impl From<serde_json::Error> for EventSelectionError {
	fn from(error: serde_json::Error) -> Self {
		Self::MessageType(error)
	}
}

impl From<mogwai::channel::mpsc::SendError> for EventSelectionError {
	fn from(error: mogwai::channel::mpsc::SendError) -> Self {
		Self::ChannelSend(error)
	}
}

impl fmt::Display for EventSelectionError {
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

pub async fn run_page(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
	user: &UserData,
) -> Result<Event, EventSelectionError> {
	let (mut event_tx, event_rx) = mogwai::channel::mpsc::channel(1);
	let (select_tx, mut select_rx) = mogwai::channel::broadcast::bounded(1);
	let event_view = builder! {
		<div id="event_selector">
			<h1>"Event Selection"</h1>
			<ul patch:children=event_rx></ul>
		</div>
	};

	let view_data = run_view(event_view, Some(user), &[]).expect("Failed to host event selection");

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
	let mut user_bar_click_channel = view_data.user_bar_click_channel.unwrap();
	let mut selection_future = select_rx.next();
	let mut user_click_future = user_bar_click_channel.next();
	loop {
		select! {
			click_target = user_click_future => {
				if let Some(ClickTarget::Admin) = click_target {
					let page_switch_message: PageControl<Event> = PageControl::Admin;
					let page_switch_json = serde_json::to_string(&page_switch_message)?;
					ws_write.send(Message::Text(page_switch_json)).await?;
					admin::run_page(ws_write, ws_read, user).await;
				}
			}
			selection = selection_future => {
				let selection = selection.unwrap();
				let selection_message = PageControl::Event(selection.clone());
				let selection_json = serde_json::to_string(&selection_message)?;
				ws_write.send(Message::Text(selection_json)).await?;
				return Ok(selection);
			}
		}
	}
}
