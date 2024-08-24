// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::page_utils::set_page_title;
use stream_log_shared::messages::user::SelfUserData;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore_router::navigate;

#[component]
pub fn RegistrationCompleteView<G: Html>(ctx: Scope) -> View<G> {
	set_page_title("Registration Complete | Stream Log");

	let user_signal: &Signal<Option<SelfUserData>> = use_context(ctx);
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
