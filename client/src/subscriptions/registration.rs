use stream_log_shared::messages::user_register::{RegistrationFinalizeResponse, UsernameCheckResponse};
use sycamore::prelude::*;

#[derive(Clone)]
pub struct RegistrationData<'a> {
	pub username_check: &'a Signal<Option<UsernameCheckResponse>>,
	pub final_register: &'a Signal<Option<RegistrationFinalizeResponse>>,
}

impl<'a> RegistrationData<'a> {
	pub fn new(ctx: Scope<'_>) -> Self {
		Self {
			username_check: create_signal(ctx, None),
			final_register: create_signal(ctx, None),
		}
	}
}
