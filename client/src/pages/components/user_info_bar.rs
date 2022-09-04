use gloo_net::websocket::futures::WebSocket;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

#[derive(Clone, Copy)]
pub enum UserBarAdminMenuClick {
	EditEvents,
	EditUsers,
	EditPermissionGroups,
	AssignUsersToPermissionGroups,
}

#[derive(Prop)]
pub struct UserInfoProps<'a> {
	pub user_data: Option<&'a UserData>,
	pub click_signal: RcSignal<Option<UserBarAdminMenuClick>>,
}

#[component]
pub fn UserInfoBar<G: Html>(ctx: Scope, user_info_props: UserInfoProps) -> View<G> {
	if let Some(user) = user_info_props.user_data {
		let username = user.username.clone();
		let is_admin = user.is_admin;
		let click_signal = user_info_props.click_signal;
		view! {
			ctx,
			div(id="user") {
				span(id="user_greeting") {
					"Hi, "
					(username)
				}
				(if is_admin {
					view! {
						ctx,
						span(id="user_admin_menu") {
							"Admin Menu"
							ul(id="user_admin_menu_pages") {
								li {
									a(
										class="click",
										on:click={
											let click_signal = click_signal.clone();
											move |_| click_signal.set(Some(UserBarAdminMenuClick::EditEvents))
										}
									) {
										"Manage Events"
									}
								}
								li {
									a(
										class="click",
										on:click={
											let click_signal = click_signal.clone();
											move |_| click_signal.set(Some(UserBarAdminMenuClick::EditUsers))
										}
									) {
										"Manage Users"
									}
								}
								li {
									a(
										class="click",
										on:click={
											let click_signal = click_signal.clone();
											move |_| click_signal.set(Some(UserBarAdminMenuClick::EditPermissionGroups))
										}
									) {
										"Manage Permission Groups"
									}
								}
								li {
									a(
										class="click",
										on:click={
											let click_signal = click_signal.clone();
											move |_| click_signal.set(Some(UserBarAdminMenuClick::AssignUsersToPermissionGroups))
										}
									) {
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
	}
}

/// Loads and renders the appropriate admin page based on the specified click target. This function renders
/// a new page, so non-admin pages should be ready to rerender their contents once the admin page logic is
/// complete.
pub async fn handle_admin_menu_click(target: UserBarAdminMenuClick, user: &UserData, ws: &mut WebSocket) {
	match target {
		UserBarAdminMenuClick::EditEvents => todo!(),
		UserBarAdminMenuClick::EditUsers => todo!(),
		UserBarAdminMenuClick::EditPermissionGroups => todo!(),
		UserBarAdminMenuClick::AssignUsersToPermissionGroups => todo!(),
	}
}
