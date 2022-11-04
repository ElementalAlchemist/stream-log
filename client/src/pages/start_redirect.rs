use stream_log_shared::messages::user::UserData;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore_router::navigate;

#[component]
pub fn StartRedirectView<G: Html>(ctx: Scope) -> View<G> {
	log::debug!("Activating start page redirect view");

	spawn_local_scoped(ctx, async move {
		let user_signal: &Signal<Option<UserData>> = use_context(ctx);

		if user_signal.get().is_some() {
			log::debug!("Redirecting to events");
			navigate("/events");
		} else {
			log::debug!("Redirecting to register");
			navigate("/register");
		}
	});

	view! { ctx, }
}
