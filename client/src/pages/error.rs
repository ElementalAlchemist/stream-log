use sycamore::prelude::*;

/// Renders an error message view
pub fn error_message_view<G: Html>(
	ctx: Scope,
	message: String,
	error: Option<impl std::fmt::Display + 'static>, // We need to ensure `error` is an owned value (if passed) so it lives long enough
) -> View<G> {
	if let Some(error) = error {
		view! {
			ctx,
			div(class="error") {
				(message)
				br {}
				(error)
			}
		}
	} else {
		view! {
			ctx,
			div(class="error") {
				(message)
			}
		}
	}
}
