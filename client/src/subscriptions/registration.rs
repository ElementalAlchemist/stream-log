// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use stream_log_shared::messages::user_register::{RegistrationFinalizeResponse, UsernameCheckResponse};
use sycamore::prelude::*;

#[derive(Clone)]
pub struct RegistrationData {
	pub username_check: RcSignal<Option<UsernameCheckResponse>>,
	pub final_register: RcSignal<Option<RegistrationFinalizeResponse>>,
}

impl RegistrationData {
	pub fn new() -> Self {
		Self {
			username_check: create_rc_signal(None),
			final_register: create_rc_signal(None),
		}
	}
}
