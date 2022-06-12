use crate::dom::run_view;
use crate::error::PageError;
use crate::user_info_bar::{UserBarBuildData, UserClickTarget};
use crate::websocket::read_websocket;
use futures::stream::{SplitSink, SplitStream};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use mogwai::prelude::*;
use stream_log_shared::messages::admin::{AdminAction, UnapprovedUsers};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataMessage, SubPageControl};

enum ClickTarget {
	Return,
	Approve(UserData),
	Deny(UserData),
}

pub async fn run_page(
	ws_write: &mut SplitSink<WebSocket, Message>,
	ws_read: &mut SplitStream<WebSocket>,
) -> Result<(), PageError> {
	/*
	This page could be written such that the DOM is manipulated precisely (e.g. once an approve/deny
	button is clicked, we remove that line and keep going). However, reloading the list each time gives
	us an approximation of real-time updates without implementing a real-time data stream for user
	approvals and registrations, which is excessive for an admin screen and comes with some UX hurdles.
	*/
	loop {
		let message = SubPageControl::Event(AdminAction::UnapprovedUserList);
		let message_json = serde_json::to_string(&message)?;
		ws_write.send(Message::Text(message_json)).await?;
		let unapproved_users: DataMessage<UnapprovedUsers> = read_websocket(ws_read).await?;
		let mut unapproved_users = unapproved_users?;

		let (click_tx, mut click_rx) = mogwai::channel::mpsc::channel(1);

		let unapproved_users_view_parts: Vec<ViewBuilder<Dom>> = unapproved_users
			.users
			.drain(..)
			.map(|user| {
				builder! {
					<div class="admin-unapproved-user">
						<a
							class="click admin-unapproved-user-approve"
							on:click=click_tx.sink().contra_map({
								let user = user.clone();
								move |_| {
									ClickTarget::Approve(user.clone())
								}
							})
						>
							"Approve"
						</a>
						<a
							class="click admin-unapproved-user-deny"
							on:click=click_tx.sink().contra_map({
								let user = user.clone();
								move |_| {
									ClickTarget::Deny(user.clone())
								}
							})
						>
							"Deny"
						</a>
						<span class="admin-unapproved-user-name">{user.username.clone()}</span>
					</div>
				}
			})
			.collect();

		let user_approval_view = builder! {
			<div id="admin-user-approval">
				<h1>"Unapproved Users"</h1>
				<a
					id="admin-return-menu"
					class="click"
					on:click=click_tx.sink().contra_map(|_| ClickTarget::Return)
				>
					"Return to Admin Menu"
				</a>
				<div id="admin-unapproved-users">
					{unapproved_users_view_parts}
				</div>
			</div>
		};

		let user_bar_build_data: Option<UserBarBuildData<UserClickTarget>> = None;
		run_view(user_approval_view, user_bar_build_data).expect("Failed to run admin unapproved users view");

		let click_target = click_rx.next().await.expect("Channel closed unexpectedly");
		match click_target {
			ClickTarget::Return => return Ok(()),
			ClickTarget::Approve(user) => {
				let message = SubPageControl::Event(AdminAction::ApproveUser(user.clone()));
				let message_json = serde_json::to_string(&message)?;
				ws_write.send(Message::Text(message_json)).await?;
			}
			ClickTarget::Deny(user) => {
				let message = SubPageControl::Event(AdminAction::DenyUser(user.clone()));
				let message_json = serde_json::to_string(&message)?;
				ws_write.send(Message::Text(message_json)).await?;
			}
		}
	}
}
