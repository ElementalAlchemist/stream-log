use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::admin::AdminTagUpdate;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AdminManageTagsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscriptions_result = {
		let subscriptions = vec![
			SubscriptionType::AdminTags,
			SubscriptionType::AdminEvents,
			SubscriptionType::AdminTagEvents,
		];
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager.add_subscriptions(subscriptions, &mut ws).await
	};
	if let Err(error) = add_subscriptions_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send tag subscription message.",
			error,
		));
	}

	let all_events = create_memo(ctx, || (*data.all_events.get()).clone());

	let event_name_index_signal = create_memo(ctx, || {
		let index: HashMap<String, Event> = data
			.all_events
			.get()
			.iter()
			.map(|event| (event.name.clone(), event.clone()))
			.collect();
		index
	});
	let selected_event_signal: &Signal<Option<Event>> = create_signal(ctx, None);
	let entered_event_name_signal = create_signal(ctx, String::new());
	let entered_event_name_error_signal = create_signal(ctx, String::new());
	let entered_event_name_has_error_signal = create_memo(ctx, || !entered_event_name_error_signal.get().is_empty());

	let tags_signal: &Signal<Vec<Tag>> = create_signal(ctx, Vec::new());
	let tag_names_index_signal = create_memo(ctx, || {
		let names: HashSet<String> = tags_signal.get().iter().map(|event| event.name.clone()).collect();
		names
	});
	let entered_new_tag_name_signal = create_signal(ctx, String::new());
	let entered_new_tag_description_signal = create_signal(ctx, String::new());
	let entered_new_tag_name_error_signal = create_signal(ctx, String::new());
	let entered_new_tag_name_has_error_signal =
		create_memo(ctx, || !entered_new_tag_name_error_signal.get().is_empty());

	let all_tags_by_id_signal = create_memo(ctx, || {
		let id_index: HashMap<String, Tag> = data
			.all_tags
			.get()
			.iter()
			.map(|tag| (tag.id.clone(), tag.clone()))
			.collect();
		id_index
	});
	let all_tags_by_event_signal = create_memo(ctx, || {
		let mut tags_by_event: HashMap<String, Vec<Tag>> = HashMap::new();
		all_tags_by_id_signal.track();
		for association in data.tag_event_associations.get().iter() {
			let Some(tag) = all_tags_by_id_signal.get().get(&association.tag).cloned() else { continue; };
			tags_by_event.entry(association.event.clone()).or_default().push(tag);
		}
		tags_by_event
	});

	let event_selection_handler = |event: WebEvent| {
		event.prevent_default();

		let entered_name = (*entered_event_name_signal.get()).clone();
		let selected_event = event_name_index_signal.get().get(&entered_name).cloned();
		entered_event_name_error_signal.set(String::new());

		if selected_event.is_none() && !entered_name.is_empty() {
			entered_event_name_error_signal.set(String::from("No event has that name"));
		}
		selected_event_signal.set(selected_event);
	};

	let add_tag_handler = move |event: WebEvent| {
		event.prevent_default();

		let for_event = if let Some(event) = selected_event_signal.get().as_ref() {
			event.clone()
		} else {
			return;
		};
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
		};

		entered_new_tag_name_signal.modify().clear();
		entered_new_tag_description_signal.modify().clear();

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminTagsUpdate(
				AdminTagUpdate::AddTag(new_tag, for_event.clone()),
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

	create_effect(ctx, move || {
		all_tags_by_event_signal.track();
		let new_selection = match selected_event_signal.get().as_ref() {
			Some(event) => event.clone(),
			None => {
				tags_signal.modify().clear();

				// Also clear out new tag form stuff
				entered_new_tag_name_signal.set(String::new());
				entered_new_tag_description_signal.set(String::new());
				entered_new_tag_name_error_signal.set(String::new());
				return;
			}
		};

		let new_tags = match all_tags_by_event_signal.get().get(&new_selection.id) {
			Some(tags) => tags.clone(),
			None => Vec::new(),
		};
		tags_signal.set(new_tags);
	});

	view! {
		ctx,
		form(id="admin_manage_tags_event_selection", on:submit=event_selection_handler) {
			datalist(id="event_names") {
				Keyed(
					iterable=all_events,
					key=|event| event.id.clone(),
					view=|ctx, event| {
						view! {
							ctx,
							option(value=&event.name)
						}
					}
				)
			}
			input(placeholder="Event name", bind:value=entered_event_name_signal, class=if *entered_event_name_has_error_signal.get() { "error" } else { "" }, list="event_names")
			button(type="submit") { "Load" }
			span(class="input_error") { (entered_event_name_error_signal.get()) }
		}
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

					let description_submission_handler = {
						let tag = tag.clone();
						move |event: WebEvent| {
							event.prevent_default();

							let new_description = (*entered_description_signal.get()).clone();
							let mut updated_tag = tag.clone();
							updated_tag.description = new_description;

							let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminTagsUpdate(AdminTagUpdate::UpdateTag(updated_tag))));
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

					let started_removal_signal = create_signal(ctx, false);

					let remove_button_clicked = |_event: WebEvent| {
						started_removal_signal.set(true);
					};
					let really_remove_button_clicked = {
						let tag = tag.clone();
						move |_event: WebEvent| {
							let tag = tag.clone();

							spawn_local_scoped(ctx, async move {
								let mut modify_tags = tags_signal.modify();
								let Some((tag_index, _)) = modify_tags.iter().enumerate().find(|(_, t)| tag.id == t.id) else { return; };
								modify_tags.remove(tag_index);

								let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminTagsUpdate(AdminTagUpdate::RemoveTag(tag))));
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

					let entered_replacement_tag_signal = create_signal(ctx, String::new());
					let entered_replacement_tag_error_signal = create_signal(ctx, String::new());
					let entered_replacement_tag_has_error_signal = create_memo(ctx, || !entered_replacement_tag_error_signal.get().is_empty());
					let replace_tag_handler = {
						let tag = tag.clone();
						move |event: WebEvent| {
							event.prevent_default();

							let tag = tag.clone();
							spawn_local_scoped(ctx, async move {
								let mut tags_list = tags_signal.modify();
								let replacement = tags_list.iter().find(|t| *entered_replacement_tag_signal.get() == t.name);
								let replacement = if let Some(replacement) = replacement {
									entered_replacement_tag_error_signal.set(String::new());
									replacement.clone()
								} else {
									entered_replacement_tag_error_signal.set(String::from("Replacement tag must exist"));
									return;
								};
								let Some((replacing_tag_index, _)) = tags_list.iter().enumerate().find(|(_, t)| tag.id == t.id) else { return; };
								tags_list.remove(replacing_tag_index);
								entered_replacement_tag_signal.set(String::new());

								let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminTagsUpdate(AdminTagUpdate::ReplaceTag(tag, replacement))));
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

					view! {
						ctx,
						tr {
							td { (tag.name) }
							td {
								form(on:submit=description_submission_handler) {
									input(type="text", class="admin_manage_tags_tag_description", bind:value=entered_description_signal, placeholder="Tag description")
									button(type="submit") { "Update" }
								}
							}
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
									button(type="submit") { "Replace Tag" }
									span(class="input_error") { (entered_replacement_tag_error_signal.get()) }
								}
							}
						}
					}
				}
			)
		}
		(if selected_event_signal.get().is_some() {
			view! {
				ctx,
				form(id="admin_manage_tags_add_new", on:submit=add_tag_handler) {
					h2 { "Add new tag" }
					div(id="admin_manage_tags_add_new_name") {
						input(bind:value=entered_new_tag_name_signal, placeholder="Tag name", class=if *entered_new_tag_name_has_error_signal.get() { "error" } else { "" })
						span(class="input_error") { (entered_new_tag_name_error_signal.get()) }
					}
					input(bind:value=entered_new_tag_description_signal, placeholder="Tag description")
					button(type="submit") { "Add tag" }
				}
			}
		} else {
			view! { ctx, }
		})
	}
}

#[component]
pub fn AdminManageTagsView<G: Html>(ctx: Scope<'_>) -> View<G> {
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
		Suspense(fallback=view! { ctx, "Loading tags manager..." }) {
			AdminManageTagsLoadedView
		}
	}
}
