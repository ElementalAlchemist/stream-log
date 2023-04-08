use std::fmt::Display;
use sycamore::prelude::*;
use web_sys::Event as WebEvent;

#[derive(Clone)]
pub struct ErrorData {
	message: &'static str,
	error: Option<String>,
}

impl ErrorData {
	/// Creates a new data object with no error object to render
	pub fn new(message: &'static str) -> Self {
		Self { message, error: None }
	}

	/// Creates a new data object with an error object to render
	pub fn new_with_error(message: &'static str, error: impl Display) -> Self {
		let error = Some(format!("{error}"));
		Self { message, error }
	}

	pub fn to_view<'a, G: Html>(&self, ctx: Scope<'a>, dismiss_handler: impl Fn(WebEvent) + 'a) -> View<G> {
		let message = self.message;
		let error_details = self.error.clone();
		view! {
			ctx,
			li(class="page_error_entry") {
				span(class="page_error_entry_text") { (message) }
				(if let Some(error_details) = error_details.clone() {
					view! {
						ctx,
						span(class="page_error_entry_details") { (error_details) }
					}
				} else {
					view! { ctx, }
				})
				span(class="page_error_entry_dismiss") {
					a(class="click", on:click=dismiss_handler) { "[X]" }
				}
			}
		}
	}
}
