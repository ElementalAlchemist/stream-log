use super::error::{ErrorData, ErrorView};
use crate::components::event_log_entry::EventLogEntryRow;
use crate::subscriptions::send_unsubscribe_all_message;
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashMap;
use stream_log_shared::messages::event_subscription::EventSubscriptionResponse;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::tags::Tag;
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

	let subscribe_response: EventSubscriptionResponse = match read_websocket(&mut ws).await {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to receive event subscription response",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let (event, permission_level, entry_types, tags, log_entries) = match subscribe_response {
		EventSubscriptionResponse::Subscribed(event, permission_level, event_types, tags, log_entries) => {
			(event, permission_level, event_types, tags, log_entries)
		}
		EventSubscriptionResponse::NoEvent => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new("That event does not exist")));
			return view! { ctx, ErrorView };
		}
		EventSubscriptionResponse::NotAllowed => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new("Not allowed to access that event")));
			return view! { ctx, ErrorView };
		}
		EventSubscriptionResponse::Error(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"An error occurred subscribing to event updates",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let event_signal = create_signal(ctx, event);
	let permission_signal = create_signal(ctx, permission_level);
	let entry_types_signal = create_signal(ctx, entry_types);
	let tags_signal = create_signal(ctx, tags);
	let log_entries = create_signal(ctx, log_entries);

	let tags_by_name_index = create_memo(ctx, || {
		let name_index: HashMap<String, Tag> = tags_signal
			.get()
			.iter()
			.map(|tag| (tag.name.clone(), tag.clone()))
			.collect();
		name_index
	});
	let can_edit = create_memo(ctx, || *permission_signal.get() == PermissionLevel::Edit);

	view! {
		ctx,
		h1(id="stream_log_event_title") { (event_signal.get().name) }
		div(id="event_log") {
			Keyed(
				iterable=log_entries,
				key=|entry| entry.id.clone(),
				view=move |ctx, entry| {
					let entry_types = entry_types_signal.get();
					let entry_type = (*entry_types).iter().find(|et| et.id == entry.entry_type).unwrap();
					let event = event_signal.get();
					let edit_open_signal = create_signal(ctx, false);
					let click_handler = if *can_edit.get() {
						Some(|| { edit_open_signal.set(true); })
					} else {
						None
					};
					view! {
						ctx,
						EventLogEntryRow(entry=entry, event=(*event).clone(), entry_type=entry_type.clone(), click_handler=click_handler)
					}
				}
			)
		}
	}
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
