use super::error::error_message_view;
use crate::websocket::read_websocket;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::events::EventSelection;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::prelude::*;

pub async fn handle_event_selection_page(user: &UserData, ws: &mut WebSocket) {
	// TODO: Accept user as UserData, or as the RcSignal taken by the user info bar?
	let message = RequestMessage::ListAvailableEvents;
	let message_json = match serde_json::to_string(&message) {
		Ok(msg) => msg,
		Err(error) => {
			sycamore::render(|ctx| {
				error_message_view(
					ctx,
					String::from("Failed to serialize event list request (critical internal error)"),
					Some(error),
				)
			});
			return;
		}
	};
	if let Err(error) = ws.send(Message::Text(message_json)).await {
		sycamore::render(|ctx| error_message_view(ctx, String::from("Failed to send event list request"), Some(error)));
		return;
	}

	let event_list_response: DataMessage<EventSelection> = match read_websocket(ws).await {
		Ok(msg) => msg,
		Err(error) => {
			sycamore::render(|ctx| {
				error_message_view(ctx, String::from("Failed to receive event list response"), Some(error))
			});
			return;
		}
	};

	let event_list = match event_list_response {
		Ok(list) => list,
		Err(error) => {
			sycamore::render(|ctx| {
				error_message_view(
					ctx,
					String::from("A server error occurred generating the event list"),
					Some(error),
				)
			});
			return;
		}
	};

	sycamore::render(|ctx| {
		let event_views = View::new_fragment(
			event_list
				.available_events
				.iter()
				.map(|event| {
					let event_name = event.name.clone();
					view! {
						ctx,
						li {
							a(
								class="click",
								on:click=|_| {
									// TODO
								}
							) {
								(event_name)
							}
						}
					}
				})
				.collect(),
		);

		view! {
			ctx,
			h1 { "Select an event" }
			ul { (event_views) }
		}
	});
}
