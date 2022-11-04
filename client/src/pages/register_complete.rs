use sycamore::prelude::*;

#[component]
pub fn RegistrationCompleteView<G: Html>(ctx: Scope) -> View<G> {
	log::debug!("Activating registration complete page");

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
