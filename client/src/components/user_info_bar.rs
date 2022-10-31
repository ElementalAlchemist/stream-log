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
					span(id="user_greeting") {
						"Hi, "
						(user.username)
					}
					(if user.is_admin {
						view! {
							ctx,
							span(id="user_admin_menu") {
								"Admin Menu"
								ul(id="user_admin_menu_pages") {
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
