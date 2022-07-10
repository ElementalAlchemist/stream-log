use sycamore::prelude::*;

pub fn handle_registration_complete() {
	sycamore::render(|ctx| {
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
	});
}
