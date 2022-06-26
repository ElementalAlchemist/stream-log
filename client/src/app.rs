use sycamore::prelude::*;

#[derive(Prop)]
pub struct AppProps<'a, G: Html> {
	page: &'a ReadSignal<View<G>>,
	user_bar: &'a ReadSignal<View<G>>,
}

#[component]
pub fn App<'a, G: Html>(ctx: Scope<'a>, props: AppProps<'a, G>) -> View<G> {
	view! {
		ctx,
		(*props.user_bar.get())
		(*props.page.get())
	}
}
