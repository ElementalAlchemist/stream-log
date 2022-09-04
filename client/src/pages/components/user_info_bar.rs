use std::collections::HashSet;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum SuppressibleUserBarParts {
	Admin,
}

#[derive(Prop)]
pub struct UserInfoProps<'a> {
	pub user_data: Option<&'a UserData>,
	pub suppress_parts: HashSet<SuppressibleUserBarParts>,
}

#[component]
pub fn UserInfoBar<G: Html>(ctx: Scope, user_info_props: UserInfoProps) -> View<G> {
	if let Some(user) = user_info_props.user_data {
		let username = user.username.clone();
		let is_admin = user.is_admin;
		let suppress_parts = user_info_props.suppress_parts;
		view! {
			ctx,
			div(id="user") {
				span(id="user_greeting") {
					"Hi, "
					(username)
				}
				(if is_admin && !suppress_parts.contains(&SuppressibleUserBarParts::Admin) {
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
		}
	} else {
		view! { ctx, }
	}
}
