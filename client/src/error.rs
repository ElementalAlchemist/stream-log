use super::dom::run_view;
use mogwai::prelude::*;
use std::fmt::Display;

pub fn render_error_message(message: &str) {
	let error_view: View<Dom> = view! {
		<div class="error">
			{message}
		</div>
	};
	run_view(error_view).expect("Failed to host view");
}

pub fn render_error_message_with_details(message: &str, error: impl Display) {
	let error_view: View<Dom> = view! {
		<div class="error">
			{message}
			<br />
			{error.to_string()}
		</div>
	};
	run_view(error_view).expect("Failed to host view");
}
