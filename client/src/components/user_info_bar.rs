// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::subscriptions::DataSignals;
use futures::future::poll_fn;
use futures::task::{Context, Poll, Waker};
use std::collections::HashMap;
use std::fmt;
use stream_log_shared::messages::user::SelfUserData;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;

pub struct EventId(String);

impl EventId {
	pub fn new(id: String) -> Self {
		Self(id)
	}
}

impl fmt::Display for EventId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let Self(id) = self;
		write!(f, "{}", id)
	}
}

#[component]
pub fn UserInfoBar<G: Html>(ctx: Scope) -> View<G> {
	let user_signal: &Signal<Option<SelfUserData>> = use_context(ctx);
	let event_id_signal: &Signal<Option<EventId>> = use_context(ctx);
	view! {
		ctx,
		(if let Some(user) = user_signal.get().as_ref().clone() {
			view! {
				ctx,
				div(id="user") {
					div(id="home_link") {
						a(href="/") {
							"Home"
						}
					}
					div(id="user_greeting") {
						"Hi, "
						(user.username)
						ul(id="user_menu", class="user_info_menu") {
							li {
								a(href="/user_profile") {
									"Profile"
								}
							}
							li {
								a(href="/logout", rel="external") {
									"Log out"
								}
							}
						}
					}
					(if let Some(event_id) = event_id_signal.get().as_ref() {
						let event_log_link = format!("/log/{}", event_id);
						let tags_link = format!("/log/{}/tags", event_id);
						let entry_types_link = format!("/log/{}/entry_types", event_id);
						view! {
							ctx,
							div(id="user_event_menu") {
								"Event Menu"
								ul(id="user_event_menu_pages", class="user_info_menu") {
									li {
										a(href=event_log_link) {
											"Event Log"
										}
									}
									li {
										a(href=tags_link) {
											"Tags"
										}
									}
									li {
										a(href=entry_types_link) {
											"Entry Types"
										}
									}
									Suspense(fallback=view! { ctx, }) {
										EventInfoPagesView
									}
								}
							}
						}
					} else {
						view! { ctx, }
					})
					(if user.is_admin {
						view! {
							ctx,
							div(id="user_admin_menu") {
								"Admin Menu"
								ul(id="user_admin_menu_pages", class="user_info_menu") {
									li {
										a(href="/admin/events") {
											"Manage Events"
										}
									}
									li {
										a(href="/admin/users") {
											"Manage Users"
										}
									}
									li {
										a(href="/admin/groups") {
											"Manage Permission Groups"
										}
									}
									li {
										a(href="/admin/assign_groups") {
											"Assign Users to Permission Groups"
										}
									}
									li {
										a(href="/admin/event_types") {
											"Manage Entry Types"
										}
									}
									li {
										a(href="/admin/assign_event_types") {
											"Assign Entry Types to Events"
										}
									}
									li {
										a(href="/admin/editors") {
											"Manage Event Editors"
										}
									}
									li {
										a(href="/admin/tabs") {
											"Manage Event Log Tabs"
										}
									}
									li {
										a(href="/admin/applications") {
											"Manage Applications"
										}
									}
									li {
										a(href="/admin/info_pages") {
											"Manage Info Pages"
										}
									}
								}
							}
						}
					} else {
						view! { ctx, }
					})
				}
			}
		} else {
			view! { ctx, }
		})
	}
}

#[component]
async fn EventInfoPagesView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let event_id_signal: &Signal<Option<EventId>> = use_context(ctx);
	let event_id = (*event_id_signal.get())
		.as_ref()
		.map(|id| id.to_string())
		.unwrap_or_default();

	let event_subscription_data = poll_fn(|poll_context: &mut Context<'_>| {
		log::debug!(
			"[User Info Bar] Checking whether event {} is present yet in the subscription manager",
			event_id
		);

		let data: &DataSignals = use_context(ctx);
		match data.events.get().get(&event_id) {
			Some(event_data) => Poll::Ready(event_data.clone()),
			None => {
				let event_wakers: &Signal<HashMap<String, Vec<Waker>>> = use_context(ctx);
				event_wakers
					.modify()
					.entry(event_id.clone())
					.or_default()
					.push(poll_context.waker().clone());
				Poll::Pending
			}
		}
	})
	.await;

	log::info!("Found info pages: {:?}", event_subscription_data.info_pages.get());

	let info_page_signal = create_memo(ctx, move || {
		let mut pages = (*event_subscription_data.info_pages.get()).clone();
		pages.sort_by(|a, b| a.title.cmp(&b.title));
		pages
	});

	view! {
		ctx,
		Keyed(
			iterable=info_page_signal,
			key=|page| page.id.clone(),
			view=move |ctx, page| {
				let url = format!("/log/{}/page/{}", event_id, page.id);
				view! {
					ctx,
					li {
						a(href=url) { (page.title) }
					}
				}
			}
		)
	}
}
