use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use stream_log_shared::messages::admin::AdminInfoPageUpdate;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::info_pages::InfoPage;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[derive(Clone)]
enum SelectedInfoPage {
	ExistingPage(InfoPage),
	NewPage,
}

#[component]
async fn AdminInfoPagesLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let set_subscription_result = {
		let subscriptions = vec![SubscriptionType::AdminEvents, SubscriptionType::AdminInfoPages];
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager.set_subscriptions(subscriptions, &mut ws).await
	};
	if let Err(error) = set_subscription_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Failed to subscribe to admin info pages.",
			error,
		));
	}

	let all_events = create_memo(ctx, || (*data.all_events.get()).clone());
	let selected_event: &Signal<Option<Event>> = create_signal(ctx, None);

	let event_info_pages = create_memo(ctx, {
		let all_info_pages = data.all_info_pages.clone();
		move || {
			let all_info_pages = all_info_pages.get();
			let Some(selected_event) = (*selected_event.get()).clone() else {
				return Vec::new();
			};
			let event_info_pages: Vec<InfoPage> = all_info_pages
				.iter()
				.filter(|page| page.event.id == selected_event.id)
				.cloned()
				.collect();
			event_info_pages
		}
	});

	let selected_page: &Signal<Option<SelectedInfoPage>> = create_signal(ctx, None);
	let selected_page_title = create_memo(ctx, || match (*selected_page.get()).as_ref() {
		Some(SelectedInfoPage::ExistingPage(page)) => event_info_pages
			.get()
			.iter()
			.find(|p| p.id == page.id)
			.map(|page| page.title.clone())
			.unwrap_or_default(),
		Some(SelectedInfoPage::NewPage) => String::new(),
		None => String::new(),
	});
	let selected_page_header = create_memo(ctx, || match (*selected_page.get()).as_ref() {
		Some(SelectedInfoPage::ExistingPage(page)) => format!("Editing: {}", page.title),
		_ => String::from("Editing new page"),
	});
	let selected_page_contents = create_memo(ctx, || match (*selected_page.get()).as_ref() {
		Some(SelectedInfoPage::ExistingPage(page)) => event_info_pages
			.get()
			.iter()
			.find(|p| p.id == page.id)
			.map(|page| page.contents.clone())
			.unwrap_or_default(),
		Some(SelectedInfoPage::NewPage) => String::new(),
		None => String::new(),
	});
	let selected_page_id = create_memo(ctx, || match (*selected_page.get()).as_ref() {
		Some(SelectedInfoPage::ExistingPage(page)) => page.id.clone(),
		_ => String::new(),
	});

	let create_new_page_handler = |_event: WebEvent| {
		selected_page.set(Some(SelectedInfoPage::NewPage));
	};

	view! {
		ctx,
		(if let Some(info_page) = selected_page.get().as_ref() {
			let info_page = info_page.clone();

			let title_entry = create_signal(ctx, (*selected_page_title.get()).clone());
			let title_error = create_memo(ctx, {
				move || {
					let title = title_entry.get();
					if title.is_empty() {
						String::from("Title cannot be empty.")
					} else if event_info_pages.get().iter().any(|page| page.title == *title && page.id != *selected_page_id.get()) {
						String::from("Another page already has this title.")
					} else {
						String::new()
					}
				}
			});

			let contents_entry = create_signal(ctx, (*selected_page_contents.get()).clone());

			let preview = create_memo(ctx, || markdown::to_html(&contents_entry.get()));

			let save_disabled = create_memo(ctx, || !title_error.get().is_empty());

			let update_page_handler = move |event: WebEvent| {
				event.prevent_default();

				let Some(selected_event) = (*selected_event.get()).clone() else {
					return; // Shouldn't have been able to get here without selecting an event
				};

				let page_title = (*title_entry.get()).clone();
				if page_title.is_empty() || event_info_pages.get().iter().any(|page| page.title == page_title && page.id != *selected_page_id.get()) {
					return; // Validation should've already caught these cases
				}

				let page_contents = (*contents_entry.get()).clone();

				let updated_info_page = InfoPage { id: (*selected_page_id.get()).clone(), event: selected_event, title: page_title, contents: page_contents };

				spawn_local_scoped(ctx, async move {
					let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
					let mut ws = ws_context.lock().await;

					let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminInfoPagesUpdate(AdminInfoPageUpdate::UpdateInfoPage(updated_info_page))));
					let message_json = match serde_json::to_string(&message) {
						Ok(msg) => msg,
						Err(error) => {
							let data: &DataSignals = use_context(ctx);
							data.errors.modify().push(ErrorData::new_with_error("Failed to serialize info page update message.", error));
							return;
						}
					};

					let send_result = ws.send(Message::Text(message_json)).await;
					if let Err(error) = send_result {
						let data: &DataSignals = use_context(ctx);
						data.errors.modify().push(ErrorData::new_with_error("Failed to send info page update message.", error));
					}

					selected_page.set(None);
				});
			};

			let cancel_handler = |_event: WebEvent| {
				selected_page.set(None);
			};

			view! {
				ctx,
				h1 { (selected_page_header.get()) }
				form(id="admin_info_pages_page_edit") {
					label {
						"Title:"
						input(bind:value=title_entry, class=if title_error.get().is_empty() { "" } else { "error" })
						span(class="input_error") { (title_error.get()) }
					}
					textarea(bind:value=contents_entry)
					h2 { "Preview" }
					div(id="admin_info_pages_page_edit_preview", dangerously_set_inner_html=&preview.get())
					div(id="admin_info_pages_page_edit_done_controls") {
						div(id="admin_info_pages_page_edit_save") {
							button(on:click=update_page_handler, disabled=*save_disabled.get()) {
								"Save"
							}
						}
						div(id="admin_info_pages_page_edit_cancel") {
							button(on:click=cancel_handler) {
								"Cancel"
							}
						}
						(if let SelectedInfoPage::ExistingPage(page) = &info_page {
							let delete_page_handler = {
								let page = page.clone();
								move |_event: WebEvent| {
									let page = page.clone();
									spawn_local_scoped(ctx, async move {
										let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
										let mut ws = ws_context.lock().await;

										let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminInfoPagesUpdate(AdminInfoPageUpdate::DeleteInfoPage(page))));
										let message_json = match serde_json::to_string(&message) {
											Ok(msg) => msg,
											Err(error) => {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to serialize info page deletion message.", error));
												return;
											}
										};

										let send_result = ws.send(Message::Text(message_json)).await;
										if let Err(error) = send_result {
											let data: &DataSignals = use_context(ctx);
											data.errors.modify().push(ErrorData::new_with_error("Failed to send info page deletion message.", error));
										}

										selected_page.set(None);
									});
								}
							};

							view! {
								ctx,
								div(id="admin_info_pages_page_edit_delete") {
									button(on:click=delete_page_handler) {
										"Delete"
									}
								}
							}
						} else {
							view! { ctx, }
						})
					}
				}
			}
		} else if let Some(event) = selected_event.get().as_ref() {
			let event_name = event.name.clone();
			let go_back_handler = |_event: WebEvent| {
				selected_event.set(None);
			};

			view! {
				ctx,
				h1 {
					"Pages for "
					(event_name)
				}
				a(class="click", on:click=go_back_handler) {
					"Back to event selection"
				}
				div(id="admin_info_pages_page_selection") {
					Keyed(
						iterable=event_info_pages,
						key=|page| page.id.clone(),
						view=move |ctx, page| {
							let page_id = page.id.clone();
							let page_title = create_memo(ctx, move || {
								let event_info_pages = event_info_pages.get();
								event_info_pages.iter().find(|page| page.id == page_id).map(|page| page.title.clone()).unwrap_or_default()
							});
							let edit_button_handler = move |_event: WebEvent| {
								selected_page.set(Some(SelectedInfoPage::ExistingPage(page.clone())));
							};

							view! {
								ctx,
								div(class="admin_info_pages_page_selection_title") {
									(page_title.get())
								}
								div(class="admin_info_pages_page_selection_edit_page") {
									button(on:click=edit_button_handler) {
										"Edit Page"
									}
								}
							}
						}
					)
					div(class="admin_info_pages_page_selection_title")
					div(class="admin_info_pages_page_selection_edit_page") {
						button(on:click=create_new_page_handler) {
							"New Page"
						}
					}
				}
			}
		} else {
			view! {
				ctx,
				div(id="admin_info_pages_event_selection") {
					Keyed(
						iterable=all_events,
						key=|event| event.id.clone(),
						view=move |ctx, event| {
							let event_name = event.name.clone();
							let edit_button_handler = move |_event: WebEvent| {
								selected_event.set(Some(event.clone()));
							};
							view! {
								ctx,
								div(class="admin_info_pages_event_selection_event_name") {
									(event_name)
								}
								div(class="admin_info_pages_event_selection_edit_pages") {
									button(on:click=edit_button_handler) {
										"Edit Pages"
									}
								}
							}
						}
					)
				}
			}
		})
	}
}

#[component]
pub fn AdminInfoPagesView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let user_signal: &Signal<Option<UserData>> = use_context(ctx);
	match user_signal.get().as_ref() {
		Some(user) => {
			if !user.is_admin {
				spawn_local_scoped(ctx, async {
					navigate("/");
				});
				return view! { ctx, };
			}
		}
		None => {
			spawn_local_scoped(ctx, async {
				navigate("/");
			});
			return view! { ctx, };
		}
	}

	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading info pages..." }) {
			AdminInfoPagesLoadedView
		}
	}
}
