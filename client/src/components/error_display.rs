use crate::subscriptions::DataSignals;
use sycamore::prelude::*;
use web_sys::Event as WebEvent;

#[component]
pub fn ErrorDisplay<G: Html>(ctx: Scope<'_>) -> View<G> {
	let data: &RcSignal<DataSignals> = use_context(ctx);
	let errors = View::new_fragment(
		data.get_untracked()
			.errors
			.get()
			.iter()
			.enumerate()
			.map(|(index, error)| {
				let dismiss_handler = |_event: WebEvent| {
					let data: &RcSignal<DataSignals> = use_context(ctx);
					data.get_untracked().errors.modify().remove(index);
				};
				error.to_view(ctx, dismiss_handler)
			})
			.collect(),
	);

	view! {
		ctx,
		ul(id="page_errors") {
			(errors)
		}
	}
}
