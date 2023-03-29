use crate::pages::error::{ErrorData, ErrorView};
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::admin::AdminAction;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AdminManageTagsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<WebSocket> = use_context(ctx);
	let mut ws = ws_context.lock().await;

	let events_request = RequestMessage::Admin(AdminAction::ListEvents);
	let events_request_json = match serde_json::to_string(&events_request) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to serialize event list request",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};
	if let Err(error) = ws.send(Message::Text(events_request_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error(
			"Failed to send event list request",
			error,
		)));
		return view! { ctx, ErrorView };
	}

	let events_response: DataMessage<Vec<Event>> = match read_websocket(&mut ws).await {
		Ok(resp) => resp,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to receive event list response",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let events = match events_response {
		Ok(events) => events,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"A server error occurred generating the events list",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let events_signal = create_signal(ctx, events);
	let event_name_index_signal = create_memo(ctx, || {
		let index: HashMap<String, Event> = events_signal
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
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = RequestMessage::Admin(AdminAction::AddTag(new_tag, for_event.clone()));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize add tag request",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error("Failed to send add tag request", error)));
				navigate("/error");
				return;
			}

			let added_tag: DataMessage<Tag> = match read_websocket(&mut ws).await {
				Ok(resp) => resp,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to receive add tag response",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			let added_tag = match added_tag {
				Ok(tag) => tag,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"A server error occurred adding a tag",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			if *selected_event_signal.get() == Some(for_event) {
				tags_signal.modify().push(added_tag);
			}
		});
	};

	create_effect(ctx, move || {
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

		tags_signal.modify().clear();
		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = RequestMessage::Admin(AdminAction::ListTagsForEvent(new_selection.clone()));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize tag list request",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send tag list request",
					error,
				)));
				navigate("/error");
				return;
			}

			let tag_list_response: DataMessage<Vec<Tag>> = match read_websocket(&mut ws).await {
				Ok(resp) => resp,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to receive tag list response",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			let tag_list = match tag_list_response {
				Ok(tags) => tags,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"A server error occurred generating the tag list",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			if *selected_event_signal.get() == Some(new_selection) {
				tags_signal.set(tag_list);
			}
		});
	});

	view! {
		ctx,
		form(id="admin_manage_tags_event_selection", on:submit=event_selection_handler) {
			datalist(id="event_names") {
				Keyed(
					iterable=events_signal,
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

							let message = RequestMessage::Admin(AdminAction::UpdateTagDescription(updated_tag));
							let message_json = match serde_json::to_string(&message) {
								Ok(msg) => msg,
								Err(error) => {
									let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
									error_signal.set(Some(ErrorData::new_with_error("Failed to serialize tag description update request", error)));
									navigate("/error");
									return;
								}
							};

							spawn_local_scoped(ctx, async move {
								let ws_context: &Mutex<WebSocket> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								if let Err(error) = ws.send(Message::Text(message_json)).await {
									let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
									error_signal.set(Some(ErrorData::new_with_error("Failed to send tag description update request", error)));
									navigate("/error");
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

								let ws_context: &Mutex<WebSocket> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								let message = RequestMessage::Admin(AdminAction::RemoveTag(tag));
								let message_json = match serde_json::to_string(&message) {
									Ok(msg) => msg,
									Err(error) => {
										let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
										error_signal.set(Some(ErrorData::new_with_error("Failed to serialize tag deletion request", error)));
										navigate("/error");
										return;
									}
								};

								if let Err(error) = ws.send(Message::Text(message_json)).await {
									let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
									error_signal.set(Some(ErrorData::new_with_error("Failed to send tag deletion request", error)));
									navigate("/error");
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

								let message = RequestMessage::Admin(AdminAction::ReplaceTag(tag, replacement));
								let message_json = match serde_json::to_string(&message) {
									Ok(msg) => msg,
									Err(error) => {
										let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
										error_signal.set(Some(ErrorData::new_with_error("Failed to serialize tag replacement request", error)));
										navigate("/error");
										return;
									}
								};

								let ws_context: &Mutex<WebSocket> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								if let Err(error) = ws.send(Message::Text(message_json)).await {
									let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
									error_signal.set(Some(ErrorData::new_with_error("Failed to send tag replacement request", error)));
									navigate("/error");
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
