// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::subscriptions::DataSignals;
use stream_log_shared::messages::user::UserData;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore_router::navigate;

#[component]
pub fn EventSelectionView<G: Html>(ctx: Scope<'_>) -> View<G> {
	{
		let user_signal: &Signal<Option<UserData>> = use_context(ctx);
		if user_signal.get().is_none() {
			spawn_local_scoped(ctx, async {
				navigate("/register");
			});
			return view! { ctx, };
		}
	}

	let data: &DataSignals = use_context(ctx);
	let available_events = create_memo(ctx, || (*data.available_events.get()).clone());

	view! {
		ctx,
		h1 { "Select an event" }
		ul {
			Keyed(
				iterable=available_events,
				key=|event| event.id.clone(),
				view=|ctx, event| {
					let event_url = format!("/log/{}", event.id);
					view! {
						ctx,
						li {
							a(href=event_url) {
								(event.name)
							}
						}
					}
				}
			)
		}
	}
}
