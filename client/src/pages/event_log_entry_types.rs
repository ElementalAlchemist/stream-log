use crate::color_utils::rgb_str_from_color;
use crate::event_type_colors::use_white_foreground;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::DataSignals;
use futures::future::poll_fn;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::task::{Context, Poll, Waker};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashMap;
use stream_log_shared::messages::subscriptions::SubscriptionType;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;

#[derive(Prop)]
pub struct EventLogEntryTypesProps {
	id: String,
}

#[component]
async fn EventLogEntryTypesLoadedView<G: Html>(ctx: Scope<'_>, props: EventLogEntryTypesProps) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let subscription_data = {
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager
			.set_subscription(SubscriptionType::EventLogData(props.id.clone()), &mut ws)
			.await
	};
	if let Err(error) = subscription_data {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send event subscription message.",
			error,
		));
	}

	let event_subscription_data = poll_fn(|poll_context: &mut Context<'_>| {
		log::debug!(
			"Checking whether event {} is present yet in the subscription manager",
			props.id
		);
		match data.events.get().get(&props.id) {
			Some(event_subscription_data) => Poll::Ready(event_subscription_data.clone()),
			None => {
				let event_wakers: &Signal<HashMap<String, Waker>> = use_context(ctx);
				event_wakers
					.modify()
					.insert(props.id.clone(), poll_context.waker().clone());
				Poll::Pending
			}
		}
	})
	.await;

	let event_entry_types = create_memo(ctx, move || (*event_subscription_data.entry_types.get()).clone());

	let event_log_url = format!("/log/{}", props.id);

	view! {
		ctx,
		a(href=event_log_url) { "⬅️ Return to event log" }
		table(id="event_log_entry_type_list") {
			Keyed(
				iterable=event_entry_types,
				key=|entry_type| entry_type.id.clone(),
				view=|ctx, entry_type| {
					let entry_type_background = rgb_str_from_color(entry_type.color);
					let entry_type_foreground = if use_white_foreground(&entry_type.color) {
						"#ffffff"
					} else {
						"#000000"
					};
					let entry_type_style = format!("background: {}; color: {}", entry_type_background, entry_type_foreground);

					view! {
						ctx,
						tr {
							td(class="entry_type_list_name", style=entry_type_style) {
								(entry_type.name)
							}
							td(class="entry_type_list_description") {
								(entry_type.description)
							}
						}
					}
				}
			)
		}
	}
}

#[component]
pub fn EventLogEntryTypesView<G: Html>(ctx: Scope<'_>, props: EventLogEntryTypesProps) -> View<G> {
	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading entry types..." }) {
			EventLogEntryTypesLoadedView(id=props.id)
		}
	}
}
