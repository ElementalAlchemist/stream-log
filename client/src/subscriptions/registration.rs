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
