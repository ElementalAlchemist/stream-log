// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::subscriptions::connection::ConnectionState;
use crate::subscriptions::DataSignals;
use sycamore::prelude::*;
use web_sys::Event as WebEvent;

#[component]
pub fn ErrorDisplay<G: Html>(ctx: Scope<'_>) -> View<G> {
	let data: &DataSignals = use_context(ctx);
	let errors = create_memo(ctx, || (*data.errors.get()).clone());
	let connection_state = create_memo(ctx, || *data.connection_state.get());

	view! {
		ctx,
		ul(id="page_errors") {
			(match *connection_state.get() {
				ConnectionState::Connected => view! { ctx, },
				ConnectionState::Reconnecting => view! { ctx, li(class="page_error_entry_connection_reconnecting") { "Connection to server lost. Reconnecting..." } },
				ConnectionState::Lost => view! { ctx, li(class="page_error_entry_connection_lost") { "Connection to server lost." } }
			})
			Indexed(
				iterable=errors,
				view=|ctx, error| {
					let dismiss_handler = {
						let error = error.clone();
						move |_event: WebEvent| {
							let data: &DataSignals = use_context(ctx);
							let index = data.errors.get().iter().enumerate().find(|(_, check_error)| error == **check_error).map(|(index, _)| index);
							if let Some(index) = index {
								data.errors.modify().remove(index);
							}
						}
					};
					error.to_view(ctx, dismiss_handler)
				}
			)
		}
	}
}
