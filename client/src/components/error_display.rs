use crate::subscriptions::DataSignals;
use sycamore::prelude::*;
use web_sys::Event as WebEvent;

#[component]
pub fn ErrorDisplay<G: Html>(ctx: Scope<'_>) -> View<G> {
	let data: &DataSignals = use_context(ctx);
	let errors = create_memo(ctx, || (*data.errors.get()).clone());

	view! {
		ctx,
		ul(id="page_errors") {
			Indexed(
				iterable=errors,
				view=|ctx, error| {
					let dismiss_handler = {
						let error = error.clone();
						move |_event: WebEvent| {
							let data: &DataSignals = use_context(ctx);
							let index = data.errors.get().iter().enumerate().find(|(_, check_error)| error == **check_error).map(|(index, _)| index);
							if let Some(index) = index {
								data.errors.modify().remove(index);
							}
						}
					};
					error.to_view(ctx, dismiss_handler)
				}
			)
		}
	}
}
