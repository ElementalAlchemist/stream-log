use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum SuppressibleUserBarParts {
	Admin,
}

pub fn render_user_info_bar<G: Html>(
	ctx: Scope,
	user: Option<&UserData>,
	suppress_user_bar_parts: &[SuppressibleUserBarParts],
) -> View<G> {
	if let Some(user) = user {
		// TODO: Do an initial user bar render with signals to control the suppressible parts
		todo!()
	} else {
		view! { ctx, }
	}
}
