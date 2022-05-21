use mogwai::prelude::*;
use stream_log_shared::messages::user::{UserApproval, UserData};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ClickTarget {
	Admin,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum SuppressibleUserBarParts {
	Admin,
}

pub struct UserBar {
	pub view: ViewBuilder<Dom>,
	pub click_channel: mpsc::Receiver<ClickTarget>,
}

pub fn user_bar(user_data: &UserData, suppress_parts: &[SuppressibleUserBarParts]) -> UserBar {
	let (click_tx, click_rx) = mpsc::channel(1);
	let user_component = builder! {
		<div id="user">
			<span id="user_greeting">
				"Hi, "
				{&user_data.username}
			</span>
			{
				if !suppress_parts.contains(&SuppressibleUserBarParts::Admin) && user_data.approval_level == UserApproval::Admin {
					Some(
						builder! {
							<a id="user_admin_link" on:click=click_tx.sink().contra_map(|_: DomEvent| ClickTarget::Admin)>
								"Admin"
							</a>
						}
					)
				} else {
					None
				}
			}
		</div>
	};
	UserBar {
		view: user_component,
		click_channel: click_rx,
	}
}
