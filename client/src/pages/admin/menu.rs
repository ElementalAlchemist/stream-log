use crate::dom::run_view;
use crate::error::PageError;
use crate::user_info_bar::{UserBarBuildData, UserClickTarget};
use futures::stream::{SplitSink, SplitStream};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use mogwai::prelude::*;
use stream_log_shared::messages::admin::AdminAction;
use stream_log_shared::messages::SubPageControl;

enum AdminMenuItem {
	Users,
	PermissionGroups,
	Events,
	Exit,
}

pub async fn run_menu(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
) -> Result<(), PageError> {
	loop {
		let (click_tx, mut click_rx) = mogwai::channel::mpsc::channel(1);

		let menu: ViewBuilder<Dom> = builder! {
			<div id="admin-menu">
				<h1>"Administration"</h1>
				<ul id="admin-menu-links">
					<li>
						<a
							class="click"
							on:click=click_tx.sink().contra_map(|_| AdminMenuItem::Users)
						>
							"User Permissions"
						</a>
					</li>
					<li>
						<a
							class="click"
							on:click=click_tx.sink().contra_map(|_| AdminMenuItem::PermissionGroups)
						>
							"Permission Groups"
						</a>
					</li>
					<li>
						<a
							class="click"
							on:click=click_tx.sink().contra_map(|_| AdminMenuItem::Events)
						>
							"Edit Events"
						</a>
					</li>
					<li>
						<a
							class="click"
							on:click=click_tx.sink().contra_map(|_| AdminMenuItem::Exit)
						>
							"Exit"
						</a>
					</li>
				</ul>
			</div>
		};

		let user_bar_build_data: Option<UserBarBuildData<UserClickTarget>> = None;
		run_view(menu, user_bar_build_data).expect("Failed to host admin menu");

		let clicked_option = click_rx.next().await.unwrap();
		match clicked_option {
			AdminMenuItem::Users => todo!(),
			AdminMenuItem::PermissionGroups => todo!(),
			AdminMenuItem::Events => todo!(),
			AdminMenuItem::Exit => {
				let request: SubPageControl<AdminAction> = SubPageControl::ReturnFromPage;
				let request_json = serde_json::to_string(&request)?;
				ws_write.send(Message::Text(request_json)).await?;
				return Ok(());
			}
		}
	}
}
