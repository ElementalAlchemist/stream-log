use sycamore::prelude::*;

#[derive(Prop)]
pub struct EventLogProps {
	id: String
}

#[component]
pub fn EventLogView<G: Html>(ctx: Scope<'_>, props: EventLogProps) -> View<G> {
	todo!()
}