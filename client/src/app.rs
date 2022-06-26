use super::user_info_bar::{UserInfoBar, UserInfoProps};
use sycamore::prelude::*;

#[derive(Prop)]
pub struct AppProps<'a, G: Html> {
	page: &'a ReadSignal<View<G>>,
	user_bar: UserInfoProps,
}

#[component]
pub fn App<'a, G: Html>(ctx: Scope<'a>, props: AppProps<'a, G>) -> View<G> {
	view! {
		ctx,
		UserInfoBar(props.user_bar)
		(*props.page.get())
	}
}
