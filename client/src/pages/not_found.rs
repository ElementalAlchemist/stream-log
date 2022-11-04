use sycamore::prelude::*;

#[component]
pub fn NotFoundView<G: Html>(ctx: Scope) -> View<G> {
	log::debug!("Activating fallback page for unknown location");

	view! {
		ctx,
		h1 { "Not found!" }
		p { "I'm not sure how you found this link or navigated to this page, but it's certainly not a real place." }
		p {
			a(href="/") {
				"Return to the main page?"
			}
		}
	}
}
