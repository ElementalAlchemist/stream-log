use super::error::ErrorData;
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::events::EventSelection;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::prelude::*;
use sycamore_router::navigate;

#[component]
pub async fn EventSelectionView<G: Html>(ctx: Scope<'_>) -> View<G> {
	log::debug!("Activating event selection page");

	let message = RequestMessage::ListAvailableEvents;
	let message_json = match serde_json::to_string(&message) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to serialize event list request (critical internal error)"),
				error,
			)));
			navigate("/error");
			return view! { ctx, };
		}
	};

	let event_list_response: DataMessage<EventSelection> = {
		let ws_context: &Mutex<WebSocket> = use_context(ctx);
		let mut ws = ws_context.lock().await;
		if let Err(error) = ws.send(Message::Text(message_json)).await {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to send event list request"),
				error,
			)));
			navigate("/error");
			return view! { ctx, };
		}

		match read_websocket(&mut ws).await {
			Ok(msg) => msg,
			Err(error) => {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("Failed to receive event list response"),
					error,
				)));
				navigate("/error");
				return view! { ctx, };
			}
		}
	};

	let event_list = match event_list_response {
		Ok(list) => list,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("A server error occurred generating the event list"),
				error,
			)));
			navigate("/error");
			return view! { ctx, };
		}
	};

	let event_views = View::new_fragment(
		event_list
			.available_events
			.iter()
			.map(|event| {
				let event = event.clone();
				let event_name = event.name.clone();
				let event_url = format!("/log/{}", event.id);
				view! {
					ctx,
					li {
						a(href=event_url) {
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
}
