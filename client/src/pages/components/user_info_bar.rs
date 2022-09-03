use std::collections::HashSet;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum SuppressibleUserBarParts {
	Admin,
}

#[derive(Prop)]
pub struct UserInfoProps {
	pub user_signal: RcSignal<Option<UserData>>,
	pub suppress_parts_signal: RcSignal<HashSet<SuppressibleUserBarParts>>,
}

#[component]
pub fn UserInfoBar<G: Html>(ctx: Scope, user_info_props: UserInfoProps) -> View<G> {
	view! {
		ctx,
		div(id="user") {
			(if let Some(user) = (*user_info_props.user_signal.get()).clone() {
				let suppress_parts = user_info_props.suppress_parts_signal.get();
				view! {
					ctx,
					span(id="user_greeting") {
						"Hi, "
						(user.username)
					}
					(if user.is_admin && !suppress_parts.contains(&SuppressibleUserBarParts::Admin) {
						view! {
							ctx,
							a(id="user_admin_link", on:click=|_| todo!()) {
								"Admin"
							}
						}
					} else {
						view! { ctx, }
					})
				}
			} else {
				view! { ctx, }
			})
		}
	}
}
