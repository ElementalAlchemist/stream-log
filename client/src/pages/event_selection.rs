use super::error::{ErrorData, ErrorView};
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::events::EventSelection;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;

#[component]
async fn EventSelectionLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	{
		let user_signal: &Signal<Option<UserData>> = use_context(ctx);
		if user_signal.get().is_none() {
			spawn_local_scoped(ctx, async {
				navigate("/register");
			});
			return view! { ctx, };
		}
	}

	let message = RequestMessage::ListAvailableEvents;
	let message_json = match serde_json::to_string(&message) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to serialize event list request (critical internal error)",
				error,
			)));
			return view! { ctx, };
		}
	};

	let event_list_response: DataMessage<EventSelection> = {
		let ws_context: &Mutex<WebSocket> = use_context(ctx);
		let mut ws = ws_context.lock().await;

		if let Err(error) = ws.send(Message::Text(message_json)).await {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to send event list request",
				error,
			)));
			return view! { ctx, ErrorView };
		}

		match read_websocket(&mut ws).await {
			Ok(msg) => msg,
			Err(error) => {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to receive event list response",
					error,
				)));
				return view! { ctx, ErrorView };
			}
		}
	};

	let event_list = match event_list_response {
		Ok(list) => list,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"A server error occurred generating the event list",
				error,
			)));
			return view! { ctx, ErrorView };
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

#[component]
pub fn EventSelectionView<G: Html>(ctx: Scope<'_>) -> View<G> {
	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading events..." }) {
			EventSelectionLoadedView
		}
	}
}
