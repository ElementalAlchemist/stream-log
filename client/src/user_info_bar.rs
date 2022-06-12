use mogwai::channel::mpsc::Sender;
use mogwai::prelude::*;
use std::marker::Unpin;
use stream_log_shared::messages::user::UserData;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum UserClickTarget {
	Admin,
}

pub trait UserBarClick {
	fn make_user_click(user_click_target: UserClickTarget) -> Self;
}

/// Implementation of the trait for the user bar's click target enum.
/// This implementation is meant to be used as a default case, useful for
/// cases where you don't want to render a user bar and don't otherwise have
/// a click target type to pass in.
impl UserBarClick for UserClickTarget {
	fn make_user_click(user_click_target: UserClickTarget) -> Self {
		user_click_target
	}
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum SuppressibleUserBarParts {
	Admin,
}

pub struct UserBarBuildData<'a, T> {
	pub user: &'a UserData,
	pub suppress_parts: &'a [SuppressibleUserBarParts],
	pub click_tx: Sender<T>,
}

pub fn user_bar<T>(build_data: UserBarBuildData<T>) -> ViewBuilder<Dom>
where
	T: UserBarClick + Unpin + Sync + Send + 'static,
	// static should be acceptable here, as we don't generally expect to be passing references through the channel
{
	builder! {
		<div id="user">
			<span id="user_greeting">
				"Hi, "
				{&build_data.user.username}
			</span>
			{
				if !build_data.suppress_parts.contains(&SuppressibleUserBarParts::Admin) && build_data.user.is_admin {
					let click_tx = build_data.click_tx;
					Some(
						builder! {
							<a
								id="user_admin_link"
								on:click=click_tx
									.sink()
									.contra_map(|_: DomEvent| T::make_user_click(UserClickTarget::Admin))
							>
								"Admin"
							</a>
						}
					)
				} else {
					None
				}
			}
		</div>
	}
}
