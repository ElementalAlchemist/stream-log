use crate::pages::components::admin_menu::{AdminMenu, AdminMenuClicked};
use crate::pages::components::user_info_bar::{SuppressibleUserBarParts, UserInfoBar};
use crate::pages::error::error_message_view;
use futures::channel::mpsc;
use futures::StreamExt;
use gloo_net::websocket::futures::WebSocket;
use std::collections::HashSet;
use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

enum PageEvent {
	MenuClick(AdminMenuClicked),
}

pub async fn handle_menu_page(user: &UserData, ws: &mut WebSocket) {
	let menu_click_signal: RcSignal<Option<AdminMenuClicked>> = create_rc_signal(None);
	let mut suppressible_user_bar_parts: HashSet<SuppressibleUserBarParts> = HashSet::new();
	suppressible_user_bar_parts.insert(SuppressibleUserBarParts::Admin);
	let (event_tx, mut event_rx) = mpsc::unbounded();

	let menu_click_signal_render = menu_click_signal.clone();
	sycamore::render(|ctx| {
		create_effect(ctx, move || {
			if let Some(click) = *menu_click_signal_render.get() {
				if let Err(error) = event_tx.unbounded_send(PageEvent::MenuClick(click)) {
					sycamore::render(|ctx| {
						error_message_view(ctx, String::from("Failed to handle admin view messaging"), Some(error))
					});
				}
			}
		});
		view! {
			ctx,
			UserInfoBar(user_data=Some(user), suppress_parts=suppressible_user_bar_parts, click_signal=create_rc_signal(None))
			div(class="admin_page") {
				AdminMenu(click_signal=menu_click_signal)
			}
		}
	});

	while let Some(event) = event_rx.next().await {
		match event {
			PageEvent::MenuClick(admin_click) => match admin_click {
				AdminMenuClicked::ManageEvents => todo!(),
				AdminMenuClicked::ManageUsers => todo!(),
				AdminMenuClicked::ManagePermissionGroups => todo!(),
				AdminMenuClicked::AssignUsersToPermissionGroups => todo!(),
				AdminMenuClicked::Exit => return,
			},
		}
	}

	let no_error: Option<String> = None;
	sycamore::render(|ctx| {
		error_message_view(
			ctx,
			String::from("Admin view internal communication ended prematurely"),
			no_error,
		)
	});
}
