use mogwai::prelude::*;
use std::fmt::Display;

pub fn render_error_message(message: &str) {
	let error_view: View<Dom> = view! {
		<div class="error">
			{message}
		</div>
	};
	error_view.run().expect("Failed to host view");
}

pub fn render_error_message_with_details(message: &str, error: impl Display) {
	let error_view: View<Dom> = view! {
		<div class="error">
			{message}
			<br />
			{error.to_string()}
		</div>
	};
	error_view.run().expect("Failed to host view");
}
