// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sycamore::prelude::*;

#[component]
pub fn NotFoundView<G: Html>(ctx: Scope) -> View<G> {
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
