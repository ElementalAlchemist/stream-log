use sycamore::prelude::*;

#[component]
pub fn AdminMenuK<G: Html>(ctx: Scope) -> View<G> {
	view! {
		ctx,
		ul(id="admin_menu") {
			li {
				a(
					class="click",
					on:click=|_| {
						todo!() // Run the admin dashboard page
					}
				) {
					"Dashboard"
				}
			}
		}
	}
}
