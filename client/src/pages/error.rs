use std::fmt::Display;
use sycamore::prelude::*;

#[derive(Clone)]
pub struct ErrorData {
	message: &'static str,
	error_display: Option<String>,
}

impl ErrorData {
	pub fn new(message: &'static str) -> Self {
		Self {
			message,
			error_display: None,
		}
	}

	pub fn new_with_error(message: &'static str, error: impl Display) -> Self {
		let error_display = Some(format!("{}", error));
		Self { message, error_display }
	}
}

#[component]
pub fn ErrorView<G: Html>(ctx: Scope) -> View<G> {
	let error_data: &Signal<Option<ErrorData>> = use_context(ctx);

	let error_message = if let Some(error) = (*error_data.get()).clone() {
		if let Some(err_disp) = error.error_display {
			return view! {
				ctx,
				div(id="app_error") {
					(error.message)
					br {}
					(err_disp)
				}
			};
		}
		error.message
	} else {
		"A completely unknown error occurred"
	};

	view! {
		ctx,
		div(id="app_error") { (error_message) }
	}
}
