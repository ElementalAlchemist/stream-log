use std::fmt::Display;
use sycamore::prelude::*;
use web_sys::Event as WebEvent;

#[derive(Clone, Eq, PartialEq)]
enum MessageString {
	Owned(String),
	Reference(&'static str),
}

impl Display for MessageString {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Owned(string) => write!(f, "{string}"),
			Self::Reference(string) => write!(f, "{string}"),
		}
	}
}

impl From<String> for MessageString {
	fn from(value: String) -> Self {
		Self::Owned(value)
	}
}

impl From<&'static str> for MessageString {
	fn from(value: &'static str) -> Self {
		Self::Reference(value)
	}
}

#[derive(Clone, Eq, PartialEq)]
pub struct ErrorData {
	message: MessageString,
	error: Option<String>,
}

impl ErrorData {
	/// Creates a new data object with no error object to render
	pub fn new(message: &'static str) -> Self {
		let message = message.into();
		Self { message, error: None }
	}

	/// Creates a new data object with an error object to render
	pub fn new_with_error(message: &'static str, error: impl Display) -> Self {
		let message = message.into();
		let error = Some(format!("{error}"));
		Self { message, error }
	}

	/// Creates a new data object with no error object from an owned string
	pub fn new_from_string(message: String) -> Self {
		let message = message.into();
		Self { message, error: None }
	}

	pub fn to_view<'a, G: Html>(&self, ctx: Scope<'a>, dismiss_handler: impl Fn(WebEvent) + 'a) -> View<G> {
		let message = self.message.clone();
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
