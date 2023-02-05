use super::error::{ErrorData, ErrorView};
use crate::subscriptions::send_unsubscribe_all_message;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::RequestMessage;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;

#[derive(Prop)]
pub struct EventLogProps {
	id: String,
}

#[component]
async fn EventLogLoadedView<G: Html>(ctx: Scope<'_>, props: EventLogProps) -> View<G> {
	let ws_context: &Mutex<WebSocket> = use_context(ctx);
	let mut ws = ws_context.lock().await;

	if let Err(error) = send_unsubscribe_all_message(&mut ws).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(error));
		return view! { ctx, ErrorView };
	}

	let subscribe_msg = RequestMessage::SubscribeToEvent(props.id.clone());
	let subscribe_msg_json = match serde_json::to_string(&subscribe_msg) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to serialize event subscription request message",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	if let Err(error) = ws.send(Message::Text(subscribe_msg_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error(
			"Failed to send event subscription request message",
			error,
		)));
		return view! { ctx, ErrorView };
	}

	todo!()
}

#[component]
pub fn EventLogView<G: Html>(ctx: Scope<'_>, props: EventLogProps) -> View<G> {
	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading event log data..." }) {
			EventLogLoadedView(id=props.id)
		}
	}
}
