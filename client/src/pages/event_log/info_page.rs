use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use futures::future::poll_fn;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::task::{Context, Poll, Waker};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashMap;
use stream_log_shared::messages::subscriptions::SubscriptionType;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;

#[derive(Prop)]
pub struct EventLogInfoPageProps {
	event_id: String,
	page_id: String,
}

#[component]
async fn EventLogInfoPageLoadedView<G: Html>(ctx: Scope<'_>, props: EventLogInfoPageProps) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let subscription_result = {
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager
			.set_subscription(SubscriptionType::EventLogData(props.event_id.clone()), &mut ws)
			.await
	};
	if let Err(error) = subscription_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send event subscription message.",
			error,
		));
	}

	let event_subscription_data = poll_fn(|poll_context: &mut Context<'_>| {
		log::debug!(
			"[Info Page] Checking whether event {} is present yet in the subscription manager",
			props.event_id
		);
		match data.events.get().get(&props.event_id) {
			Some(event_data) => Poll::Ready(event_data.clone()),
			None => {
				let wakers: &Signal<HashMap<String, Vec<Waker>>> = use_context(ctx);
				wakers
					.modify()
					.entry(props.event_id.clone())
					.or_default()
					.push(poll_context.waker().clone());
				Poll::Pending
			}
		}
	})
	.await;

	let event_info_pages = event_subscription_data.info_pages.clone();
	let info_page = create_memo(ctx, move || {
		event_info_pages
			.get()
			.iter()
			.find(|page| page.id == props.page_id)
			.cloned()
	});
	let page_title = create_memo(ctx, || {
		(*info_page.get())
			.as_ref()
			.map(|page| page.title.clone())
			.unwrap_or_default()
	});
	let page_contents = create_memo(ctx, || {
		(*info_page.get())
			.as_ref()
			.map(|page| page.contents.clone())
			.unwrap_or_default()
	});

	view! {
		ctx,
		h1 { (page_title.get()) }
		div { (page_contents.get()) }
	}
}

#[component]
pub fn EventLogInfoPageView<G: Html>(ctx: Scope<'_>, props: EventLogInfoPageProps) -> View<G> {
	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading info page..." }) {
			EventLogInfoPageLoadedView(event_id=props.event_id, page_id=props.page_id)
		}
	}
}
