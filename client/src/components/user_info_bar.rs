use std::fmt;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

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
	let user_signal: &Signal<Option<UserData>> = use_context(ctx);
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
										a(href="/admin/sections") {
											"Manage Event Log Sections"
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
