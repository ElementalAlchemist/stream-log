use super::admin;
use crate::dom::run_view;
use crate::user_info_bar::{UserBarBuildData, UserBarClick, UserClickTarget};
use crate::websocket::{read_websocket, WebSocketReadError};
use futures::stream::{SplitSink, SplitStream};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::{Message, WebSocketError};
use mogwai::channel::mpsc::Sender;
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

enum EventClickTarget {
	UserBar(UserClickTarget),
	Event(Event),
}

impl UserBarClick for EventClickTarget {
	fn make_user_click(user_click_target: UserClickTarget) -> Self {
		Self::UserBar(user_click_target)
	}
}

/// Renders the event selection page. Use this only when we're expected to receive an event
/// list from the server.
async fn render_page(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
	user: &UserData,
	select_tx: &Sender<EventClickTarget>,
) -> Result<EventSelection, EventSelectionError> {
	let event_selection: DataMessage<EventSelection> = read_websocket(ws_read).await?;
	let event_selection = event_selection?;
	let mut event_dom: Vec<ViewBuilder<Dom>> = Vec::with_capacity(event_selection.available_events.len());
	for event in event_selection.available_events.iter() {
		let event_builder = builder! {
			<li
				data-event-id=event.id.clone()
				class="event-click"
				on:click=select_tx.sink().contra_map({
					let event = event.clone();
					move |_: DomEvent| {
						EventClickTarget::Event(event.clone())
					}
				})
			>
				{event.name.clone()}
			</li>
		};
		event_dom.push(event_builder);
	}
	let event_view = builder! {
		<div id="event_selector">
			<h1>"Event Selection"</h1>
			<ul>
				{event_dom}
			</ul>
		</div>
	};

	let user_bar_build_data = Some(UserBarBuildData {
		user,
		suppress_parts: &[],
		click_tx: select_tx.clone(),
	});
	run_view(event_view, user_bar_build_data).expect("Failed to host event selection view");

	Ok(event_selection)
}

pub async fn run_page(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
	user: &UserData,
) -> Result<Event, EventSelectionError> {
	let (select_tx, mut select_rx) = mogwai::channel::mpsc::channel(1);
	let mut event_selection = render_page(ws_write, ws_read, user, &select_tx).await?;

	while let Some(selection) = select_rx.next().await {
		match selection {
			EventClickTarget::UserBar(user_click) => match user_click {
				UserClickTarget::Admin => {
					let page_switch_message: PageControl<Event> = PageControl::Admin;
					let page_switch_json = serde_json::to_string(&page_switch_message)?;
					ws_write.send(Message::Text(page_switch_json)).await?;
					admin::run_page(ws_write, ws_read, user).await;
					event_selection = render_page(ws_write, ws_read, user, &select_tx).await?;
				}
			},
			EventClickTarget::Event(selected_event) => {
				let selection_message = PageControl::Event(selected_event.clone());
				let selection_json = serde_json::to_string(&selection_message)?;
				ws_write.send(Message::Text(selection_json)).await?;
				return Ok(selected_event);
			}
		}
	}
	unreachable!("The channel should not close until the loop is exited normally")
}
