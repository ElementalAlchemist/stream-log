use std::fmt::Display;
use sycamore::prelude::*;

#[derive(Clone)]
pub struct ErrorData {
	message: String,
	error_display: Option<String>,
}

impl ErrorData {
	pub fn new(message: String) -> Self {
		Self {
			message,
			error_display: None,
		}
	}

	pub fn new_with_error(message: String, error: impl Display) -> Self {
		let error_display = Some(format!("{}", error));
		Self { message, error_display }
	}
}

#[component]
pub fn ErrorView<G: Html>(ctx: Scope) -> View<G> {
	log::debug!("Activating error page");

	let error_data: &Option<ErrorData> = use_context(ctx);

	let error_message = if let Some(error) = error_data.clone() {
		if let Some(err_disp) = error.error_display {
			return view! {
				ctx,
				div(class="error") {
					(error.message)
					br {}
					(err_disp)
				}
			};
		}
		error.message
	} else {
		String::from("A completely unknown error occurred")
	};

	view! {
		ctx,
		div(class="error") { (error_message) }
	}
}
