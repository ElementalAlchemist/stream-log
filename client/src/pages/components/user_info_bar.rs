use crate::pages::admin::run_admin_page;
use gloo_net::websocket::futures::WebSocket;
use std::collections::HashSet;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum SuppressibleUserBarParts {
	Admin,
}

#[derive(Clone, Copy)]
pub enum UserBarClick {
	Admin,
}

#[derive(Prop)]
pub struct UserInfoProps<'a> {
	pub user_data: Option<&'a UserData>,
	pub suppress_parts: HashSet<SuppressibleUserBarParts>,
	pub click_signal: RcSignal<Option<UserBarClick>>,
}

#[component]
pub fn UserInfoBar<G: Html>(ctx: Scope, user_info_props: UserInfoProps) -> View<G> {
	if let Some(user) = user_info_props.user_data {
		let username = user.username.clone();
		let is_admin = user.is_admin;
		let suppress_parts = user_info_props.suppress_parts;
		let click_signal = user_info_props.click_signal;
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
						a(
							id="user_admin_link",
							on:click={
								let click_signal = click_signal.clone();
								move |_| click_signal.set(Some(UserBarClick::Admin))
							}
						) {
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

pub async fn handle_user_bar_click(click_target: UserBarClick, user: &UserData, ws: &mut WebSocket) {
	match click_target {
		UserBarClick::Admin => run_admin_page(user, ws).await,
	}
}
