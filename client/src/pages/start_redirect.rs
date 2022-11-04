use stream_log_shared::messages::user::UserData;
use sycamore::prelude::*;
use sycamore_router::navigate;

#[component]
pub fn StartRedirectView<G: Html>(ctx: Scope) -> View<G> {
	let user_signal: &Signal<Option<UserData>> = use_context(ctx);

	if user_signal.get().is_some() {
		navigate("/events");
	} else {
		navigate("/register");
	}

	view! { ctx, }
}
