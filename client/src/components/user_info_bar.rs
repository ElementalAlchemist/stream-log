use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

#[component]
pub fn UserInfoBar<G: Html>(ctx: Scope) -> View<G> {
	let user_signal: &Signal<Option<UserData>> = use_context(ctx);
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
					div(id="user_tags_link") {
						a(href="/tags") {
							"Tags"
						}
					}
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
