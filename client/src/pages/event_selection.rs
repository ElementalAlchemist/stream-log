use super::admin;
use crate::dom::run_view;
use crate::error::PageError;
use crate::user_info_bar::{UserBarBuildData, UserBarClick, UserClickTarget};
use crate::websocket::read_websocket;
use futures::stream::{SplitSink, SplitStream};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use mogwai::channel::mpsc::Sender;
use mogwai::prelude::*;
use stream_log_shared::messages::events::{Event, EventSelection};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataMessage, PageControl};

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
) -> Result<(), PageError> {
	let event_selection: DataMessage<EventSelection> = read_websocket(ws_read).await?;
	let event_selection = event_selection?;
	let mut event_dom: Vec<ViewBuilder<Dom>> = Vec::with_capacity(event_selection.available_events.len());
	for event in event_selection.available_events.iter() {
		let event_builder = builder! {
			<li
				data-event-id=event.id.clone()
				class="click"
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
	let event_view = if event_dom.is_empty() {
		builder! {
			<div id="event_selector">
				<h1>"Event Selection"</h1>
				<p id="event_selection_empty">
					"There are no events right now."
				</p>
			</div>
		}
	} else {
		builder! {
			<div id="event_selector">
				<h1>"Event Selection"</h1>
				<ul>
					{event_dom}
				</ul>
			</div>
		}
	};

	let user_bar_build_data = Some(UserBarBuildData {
		user,
		suppress_parts: &[],
		click_tx: select_tx.clone(),
	});
	run_view(event_view, user_bar_build_data).expect("Failed to host event selection view");

	Ok(())
}

pub async fn run_page(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
	user: &UserData,
) -> Result<Event, PageError> {
	loop {
		let (select_tx, mut select_rx) = mogwai::channel::mpsc::channel(1);
		render_page(ws_write, ws_read, user, &select_tx).await?;

		let selection = select_rx.next().await.expect("Channel closed unexpectedly");
		match selection {
			EventClickTarget::UserBar(user_click) => match user_click {
				UserClickTarget::Admin => {
					let page_switch_message: PageControl<Event> = PageControl::Admin;
					let page_switch_json = serde_json::to_string(&page_switch_message)?;
					ws_write.send(Message::Text(page_switch_json)).await?;
					admin::run_page(ws_write, ws_read).await?;
					render_page(ws_write, ws_read, user, &select_tx).await?;
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
}
