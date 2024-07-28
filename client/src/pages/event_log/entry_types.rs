// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::color_utils::rgb_str_from_color;
use crate::entry_type_colors::use_white_foreground;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::websocket::WebSocketSendStream;
use crate::DataSignals;
use futures::future::poll_fn;
use futures::lock::Mutex;
use futures::task::{Context, Poll, Waker};
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
	let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
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
			"[Entry Types] Checking whether event {} is present yet in the subscription manager",
			props.id
		);
		match data.events.get().get(&props.id) {
			Some(event_subscription_data) => Poll::Ready(event_subscription_data.clone()),
			None => {
				let event_wakers: &Signal<HashMap<String, Vec<Waker>>> = use_context(ctx);
				event_wakers
					.modify()
					.entry(props.id.clone())
					.or_default()
					.push(poll_context.waker().clone());
				Poll::Pending
			}
		}
	})
	.await;

	let event_entry_types = create_memo(ctx, move || (*event_subscription_data.entry_types.get()).clone());

	view! {
		ctx,
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
