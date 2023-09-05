use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashSet;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::tags::{Tag, TagListUpdate};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use web_sys::Event as WebEvent;

#[component]
async fn TagListLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let user_signal: &Signal<Option<UserData>> = use_context(ctx);
	let user_is_admin = create_memo(ctx, || {
		(*user_signal.get()).as_ref().map(|user| user.is_admin).unwrap_or(false)
	});

	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscriptions_result = {
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager
			.set_subscription(SubscriptionType::TagList, &mut ws)
			.await
	};
	if let Err(error) = add_subscriptions_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send tag subscription message.",
			error,
		));
	}

	let tags_signal = create_memo(ctx, || (*data.all_tags.get()).clone());
	let tag_names_index_signal = create_memo(ctx, || {
		let names: HashSet<String> = tags_signal.get().iter().map(|event| event.name.clone()).collect();
		names
	});
	let entered_new_tag_name_signal = create_signal(ctx, String::new());
	let entered_new_tag_description_signal = create_signal(ctx, String::new());
	let entered_new_tag_name_error_signal = create_signal(ctx, String::new());
	let entered_new_tag_name_has_error_signal =
		create_memo(ctx, || !entered_new_tag_name_error_signal.get().is_empty());

	let add_tag_handler = move |event: WebEvent| {
		event.prevent_default();

		let name = (*entered_new_tag_name_signal.get()).clone();
		if name.is_empty() {
			entered_new_tag_name_error_signal.set(String::from("New tag must have a name"));
			return;
		}
		if tag_names_index_signal.get().contains(&name) {
			entered_new_tag_name_error_signal.set(String::from("New tag must have a unique name"));
			return;
		}

		let description = (*entered_new_tag_description_signal.get()).clone();

		let new_tag = Tag {
			id: String::new(),
			name,
			description,
			playlist: String::new(),
		};

		entered_new_tag_name_signal.modify().clear();
		entered_new_tag_description_signal.modify().clear();

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::TagListUpdate(
				TagListUpdate::UpdateTag(new_tag),
			)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors
						.modify()
						.push(ErrorData::new_with_error("Failed to serialize new tag message.", error));
					return;
				}
			};

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send new tag message.", error));
			}
		});
	};

	view! {
		ctx,
		datalist(id="tag_names") {
			Keyed(
				iterable=tags_signal,
				key=|tag| tag.id.clone(),
				view=|ctx, tag| {
					view! {
						ctx,
						option(value=&tag.name)
					}
				}
			)
		}
		table(id="admin_manage_tags_list") {
			Keyed(
				iterable=tags_signal,
				key=|tag| tag.id.clone(),
				view=move |ctx, tag| {
					let entered_description_signal = create_signal(ctx, tag.description.clone());

					let started_removal_signal = create_signal(ctx, false);

					let remove_button_clicked = |_event: WebEvent| {
						started_removal_signal.set(true);
					};

					let entered_replacement_tag_signal = create_signal(ctx, String::new());
					let entered_replacement_tag_error_signal = create_signal(ctx, String::new());
					let entered_replacement_tag_has_error_signal = create_memo(ctx, || !entered_replacement_tag_error_signal.get().is_empty());

					let description_submission_handler = {
						let tag = tag.clone();
						move |event: WebEvent| {
							event.prevent_default();

							let new_description = (*entered_description_signal.get()).clone();
							let mut updated_tag = tag.clone();
							updated_tag.description = new_description;

							let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::TagListUpdate(TagListUpdate::UpdateTag(updated_tag))));
							let message_json = match serde_json::to_string(&message) {
								Ok(msg) => msg,
								Err(error) => {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to serialize tag description update.", error));
									return;
								}
							};

							spawn_local_scoped(ctx, async move {
								let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								if let Err(error) = ws.send(Message::Text(message_json)).await {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to send tag description update.", error));
								}
							});
						}
					};

					let tag_name = tag.name.clone();
					view! {
						ctx,
						tr {
							td { (tag_name) }
							td {
								form(on:submit=description_submission_handler) {
									input(type="text", class="admin_manage_tags_tag_description", bind:value=entered_description_signal, placeholder="Tag description")
									button(type="submit") { "Update" }
								}
							}
							(if *user_is_admin.get() {
								let really_remove_button_clicked = {
									let tag = tag.clone();
									move |_event: WebEvent| {
										let tag = tag.clone();

										spawn_local_scoped(ctx, async move {
											let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
											let mut ws = ws_context.lock().await;

											let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::TagListUpdate(TagListUpdate::RemoveTag(tag))));
											let message_json = match serde_json::to_string(&message) {
												Ok(msg) => msg,
												Err(error) => {
													let data: &DataSignals = use_context(ctx);
													data.errors.modify().push(ErrorData::new_with_error("Failed to serialize tag deletion.", error));
													return;
												}
											};

											if let Err(error) = ws.send(Message::Text(message_json)).await {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to send tag deletion.", error));
											}
										});
									}
								};

								let do_not_remove_button_clicked = |_event: WebEvent| {
									started_removal_signal.set(false);
								};

								let replace_tag_handler = {
									let tag = tag.clone();
									move |event: WebEvent| {
										event.prevent_default();

										let tag = tag.clone();
										spawn_local_scoped(ctx, async move {
											let tags_list = tags_signal.get();
											let replacement = tags_list.iter().find(|t| *entered_replacement_tag_signal.get() == t.name);
											let replacement = if let Some(replacement) = replacement {
												entered_replacement_tag_error_signal.set(String::new());
												replacement.clone()
											} else {
												entered_replacement_tag_error_signal.set(String::from("Replacement tag must exist"));
												return;
											};
											entered_replacement_tag_signal.set(String::new());

											let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::TagListUpdate(TagListUpdate::ReplaceTag(tag, replacement))));
											let message_json = match serde_json::to_string(&message) {
												Ok(msg) => msg,
												Err(error) => {
													let data: &DataSignals = use_context(ctx);
													data.errors.modify().push(ErrorData::new_with_error("Failed to serialize tag replacement.", error));
													return;
												}
											};

											let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
											let mut ws = ws_context.lock().await;

											if let Err(error) = ws.send(Message::Text(message_json)).await {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to send tag replacement.", error));
											}
										});
									}
								};

								let entered_playlist_signal = create_signal(ctx, tag.playlist.clone());
								let update_playlist_handler = {
									let tag = tag.clone();
									move |event: WebEvent| {
										event.prevent_default();

										let new_playlist_id = (*entered_playlist_signal.get()).clone();
										let mut updated_tag = tag.clone();
										updated_tag.playlist = new_playlist_id;

										let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::TagListUpdate(TagListUpdate::UpdateTag(updated_tag))));
										let message_json = match serde_json::to_string(&message) {
											Ok(msg) => msg,
											Err(error) => {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to serialize tag playlist update.", error));
												return;
											}
										};

										spawn_local_scoped(ctx, async move {
											let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
											let mut ws = ws_context.lock().await;

											if let Err(error) = ws.send(Message::Text(message_json)).await {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to send tag playlist update.", error));
											}
										});
									}
								};

								view! {
									ctx,
									td {
										(if *started_removal_signal.get() {
											view! {
												ctx,
												"Removing this tag will remove all uses of this tag from the event log."
												button(type="button", on:click=really_remove_button_clicked.clone()) { "Yes, remove it!" }
												button(type="button", on:click=do_not_remove_button_clicked) { "Oh, never mind." }
											}
										} else {
											view! {
												ctx,
												button(type="button", on:click=remove_button_clicked) { "Remove" }
											}
										})
									}
									td {
										form(on:submit=replace_tag_handler) {
											input(type="text", list="tag_names", bind:value=entered_replacement_tag_signal, class=if *entered_replacement_tag_has_error_signal.get() { "error" } else { "" })
											button(type="submit", title="Replacing a tag will remove this tag and add the replacement tag where this tag was used previously.") { "Replace Tag" }
											span(class="input_error") { (entered_replacement_tag_error_signal.get()) }
										}
									}
									td {
										form(on:submit=update_playlist_handler) {
											input(type="text", bind:value=entered_playlist_signal)
											button(type="submit") { "Set playlist ID" }
										}
									}
								}
							} else {
								view! { ctx, }
							})
						}
					}
				}
			)
		}
		form(id="admin_manage_tags_add_new", on:submit=add_tag_handler) {
			h2 { "Add new tag" }
			div(id="admin_manage_tags_add_new_name") {
				input(bind:value=entered_new_tag_name_signal, placeholder="Tag name", class=if *entered_new_tag_name_has_error_signal.get() { "error" } else { "" })
				span(class="input_error") { (entered_new_tag_name_error_signal.get()) }
			}
			div(id="admin_manage_tags_add_new_description") {
				input(bind:value=entered_new_tag_description_signal, placeholder="Tag description")
			}
			div {
				button(type="submit") { "Add tag" }
			}
		}
	}
}

#[component]
pub fn TagListView<G: Html>(ctx: Scope<'_>) -> View<G> {
	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading tags manager..." }) {
			TagListLoadedView
		}
	}
}
