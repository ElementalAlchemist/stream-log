use stream_log_shared::messages::user::UserData;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore_router::navigate;

#[component]
pub fn RegistrationCompleteView<G: Html>(ctx: Scope) -> View<G> {
	let user_signal: &Signal<Option<UserData>> = use_context(ctx);
	if user_signal.get().is_none() {
		spawn_local_scoped(ctx, async {
			navigate("/register");
		});
	}

	view! {
		ctx,
		div(id="register_complete") {
			h1 {
				"Registration complete!"
			}
			p {
				"Your account has been created."
			}
			p {
				"An administrator will review your account and grant access to the appropriate events."
			}
		}
	}
}
