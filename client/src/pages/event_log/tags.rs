use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use crate::websocket::WebSocketSendStream;
use futures::future::poll_fn;
use futures::lock::Mutex;
use futures::task::{Context, Poll, Waker};
use gloo_net::websocket::Message;
use std::collections::HashMap;
use stream_log_shared::messages::event_subscription::EventSubscriptionUpdate;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::tags::Tag;
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use web_sys::Event as WebEvent;

#[derive(Prop)]
pub struct EventLogTagsProps {
	id: String,
}

#[component]
async fn EventLogTagsLoadedView<G: Html>(ctx: Scope<'_>, props: EventLogTagsProps) -> View<G> {
	let user: &Signal<Option<UserData>> = use_context(ctx);
	let user_is_admin_signal = create_memo(ctx, || {
		let user = user.get();
		match user.as_ref() {
			Some(user) => user.is_admin,
			None => false,
		}
	});

	let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscription_data = {
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		let mut subscription_list = vec![SubscriptionType::EventLogData(props.id.clone())];
		if *user_is_admin_signal.get() {
			subscription_list.push(SubscriptionType::AdminEvents);
		}
		subscription_manager.set_subscriptions(subscription_list, &mut ws).await
	};
	if let Err(error) = add_subscription_data {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send event subscription message.",
			error,
		));
	}
	let event_subscription_data = poll_fn(|poll_context: &mut Context<'_>| {
		log::debug!(
			"[Tags] Checking whether event {} is present yet in the subscription manager",
			props.id
		);
		match data.events.get().get(&props.id) {
			Some(event_subscription_data) => Poll::Ready(event_subscription_data.clone()),
			None => {
				let event_wakers: &Signal<HashMap<String, Vec<Waker>>> = use_context(ctx);
				event_wakers
					.modify()
					.entry(props.id.clone())
					.or_default()
					.push(poll_context.waker().clone());
				Poll::Pending
			}
		}
	})
	.await;

	let event_signal = event_subscription_data.event.clone();
	let permission_signal = event_subscription_data.permission.clone();
	let tags_signal = event_subscription_data.tags.clone();

	let read_events_signal = create_memo(ctx, || (*data.all_events.get()).clone());
	let read_tags_signal = create_memo(ctx, {
		let tags_signal = tags_signal.clone();
		move || (*tags_signal.get()).clone()
	});

	let event_names_index = create_memo(ctx, || {
		let event_names: HashMap<String, Event> = data
			.all_events
			.get()
			.iter()
			.map(|event| (event.name.clone(), event.clone()))
			.collect();
		event_names
	});
	let tag_names_index = create_memo(ctx, {
		let tags_signal = tags_signal.clone();
		move || {
			let tag_names: HashMap<String, Tag> = tags_signal
				.get()
				.iter()
				.map(|tag| (tag.name.clone(), tag.clone()))
				.collect();
			tag_names
		}
	});

	let can_edit_signal = create_memo(ctx, {
		let permission_signal = permission_signal.clone();
		move || permission_signal.get().can_edit()
	});
	let is_supervisor_signal = create_memo(ctx, {
		let permission_signal = permission_signal.clone();
		move || *permission_signal.get() == PermissionLevel::Supervisor
	});

	let new_event_signal = event_signal.clone();
	let copy_event_signal = event_signal.clone();

	view! {
		ctx,
		datalist(id="tags") {
			Keyed(
				iterable=read_tags_signal,
				key=|tag| tag.id.clone(),
				view=|ctx, tag| {
					view! {
						ctx,
						option(value=tag.name)
					}
				}
			)
		}
		datalist(id="events") {
			Keyed(
				iterable=read_events_signal,
				key=|event| event.id.clone(),
				view=|ctx, event| {
					view! {
						ctx,
						option(value=event.name)
					}
				}
			)
		}
		table(id="manage_tags_list") {
			tr {
				th { "Name" }
				th { "Description" }
				th { "Playlist" }
			}
			Keyed(
				iterable=read_tags_signal,
				key=|tag| tag.id.clone(),
				view=move |ctx, tag| {
					let entered_description = create_signal(ctx, tag.description.clone());
					let entered_description_error = create_signal(ctx, String::new());

					let entered_playlist = create_signal(ctx, tag.playlist.clone());

					let confirming_delete = create_signal(ctx, false);

					let entered_replacement_tag = create_signal(ctx, String::new());
					let entered_replacement_tag_error = create_signal(ctx, String::new());

					let start_delete_handler = |_event: WebEvent| {
						confirming_delete.set(true);
					};

					let cancel_delete_handler = |_event: WebEvent| {
						confirming_delete.set(false);
					};

					let description_event_signal = event_signal.clone();
					let description_tag = tag.clone();

					let handler_event_signal = event_signal.clone();
					let handler_tag = tag.clone();

					let tag_playlist = tag.playlist.clone();

					view! {
						ctx,
						tr {
							td { (tag.name) }
							td {
								(if *can_edit_signal.get() {
									let submit_description_handler = {
										let event_signal = description_event_signal.clone();
										let tag = description_tag.clone();

										move |event: WebEvent| {
											event.prevent_default();

											let event_signal = event_signal.clone();

											let description = entered_description.get();
											if description.is_empty() {
												entered_description_error.set(String::from("Description cannot be empty."));
												return;
											}
											entered_description_error.modify().clear();

											let mut tag = tag.clone();
											tag.description.clone_from(&(*description));

											spawn_local_scoped(ctx, async move {
												let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
												let mut ws = ws_context.lock().await;

												let message = FromClientMessage::SubscriptionMessage(
													Box::new(
														SubscriptionTargetUpdate::EventUpdate(
															(*event_signal.get()).clone(),
															Box::new(
																EventSubscriptionUpdate::UpdateTag(tag)
															)
														)
													)
												);
												let message_json = match serde_json::to_string(&message) {
													Ok(msg) => msg,
													Err(error) => {
														let data: &DataSignals = use_context(ctx);
														data.errors.modify().push(ErrorData::new_with_error("Failed to serialize tag description update.", error));
														return;
													}
												};

												let send_result = ws.send(Message::Text(message_json)).await;
												if let Err(error) = send_result {
													let data: &DataSignals = use_context(ctx);
													data.errors.modify().push(ErrorData::new_with_error("Failed to send tag description update.", error));
												}
											});
										}
									};

									view! {
										ctx,
										form(on:submit=submit_description_handler) {
											input(
												bind:value=entered_description,
												placeholder="Description",
												class={
													if entered_description_error.get().is_empty() {
														"manage_tags_tag_description"
													} else {
														"manage_tags_tag_description error"
													}
												},
												title=entered_description_error.get()
											)
											button(type="submit") { "Update" }
										}
									}
								} else {
									let description = tag.description.clone();
									view! {
										ctx,
										(description)
									}
								})
							}
							(if *is_supervisor_signal.get() {
								let handler_event_signal = handler_event_signal.clone();
								let handler_tag = handler_tag.clone();

								let set_playlist_handler = {
									let event_signal = handler_event_signal.clone();
									let tag = handler_tag.clone();

									move |event: WebEvent| {
										event.prevent_default();

										let event_signal = event_signal.clone();

										let playlist_id = (*entered_playlist.get()).clone();
										let mut tag = tag.clone();
										tag.playlist = playlist_id;

										spawn_local_scoped(ctx, async move {
											let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
											let mut ws = ws_context.lock().await;

											let message = FromClientMessage::SubscriptionMessage(
												Box::new(
													SubscriptionTargetUpdate::EventUpdate(
														(*event_signal.get()).clone(),
														Box::new(
															EventSubscriptionUpdate::UpdateTag(tag)
														)
													)
												)
											);
											let message_json = match serde_json::to_string(&message) {
												Ok(msg) => msg,
												Err(error) => {
													let data: &DataSignals = use_context(ctx);
													data.errors.modify().push(ErrorData::new_with_error("Failed to serialize tag playlist update message.", error));
													return;
												}
											};

											let send_result = ws.send(Message::Text(message_json)).await;
											if let Err(error) = send_result {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to send tag playlist update message.", error));
											}
										});
									}
								};

								let replace_tag_handler = {
									let event_signal = handler_event_signal.clone();
									let tag = handler_tag.clone();

									move |event: WebEvent| {
										event.prevent_default();

										let event_signal = event_signal.clone();
										let tag = tag.clone();

										let replacement_tag_name = entered_replacement_tag.get();
										if replacement_tag_name.is_empty() {
											entered_replacement_tag_error.set(String::new());
											return;
										}
										let Some(replacement_tag) = tag_names_index.get().get(&*replacement_tag_name).cloned() else {
											entered_replacement_tag_error.set(String::from("Replacement tag must exist"));
											return;
										};
										if replacement_tag.id == tag.id {
											entered_replacement_tag_error.set(String::from("Cannot replace a tag with itself"));
											return;
										}

										spawn_local_scoped(ctx, async move {
											let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
											let mut ws = ws_context.lock().await;

											let message = FromClientMessage::SubscriptionMessage(
												Box::new(
													SubscriptionTargetUpdate::EventUpdate(
														(*event_signal.get()).clone(),
														Box::new(
															EventSubscriptionUpdate::ReplaceTag(tag, replacement_tag)
														)
													)
												)
											);
											let message_json = match serde_json::to_string(&message) {
												Ok(msg) => msg,
												Err(error) => {
													let data: &DataSignals = use_context(ctx);
													data.errors.modify().push(ErrorData::new_with_error("Failed to serialize tag replacement message.", error));
													return;
												}
											};

											let send_result = ws.send(Message::Text(message_json)).await;
											if let Err(error) = send_result {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to send tag replacement message.", error));
											}
										});
									}
								};

								view! {
									ctx,
									td {
										form(on:submit=set_playlist_handler) {
											input(
												bind:value=entered_playlist,
												placeholder="Playlist ID"
											)
											button(type="submit") { "Set Playlist" }
										}
									}
									td {
										(if *confirming_delete.get() {
											let confirm_delete_handler = {
												let event_signal = handler_event_signal.clone();
												let tag = handler_tag.clone();

												move |_event: WebEvent| {
													let event_signal = event_signal.clone();
													let tag = tag.clone();

													spawn_local_scoped(ctx, async move {
														let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
														let mut ws = ws_context.lock().await;

														let message = FromClientMessage::SubscriptionMessage(
															Box::new(
																SubscriptionTargetUpdate::EventUpdate(
																	(*event_signal.get()).clone(),
																	Box::new(
																		EventSubscriptionUpdate::RemoveTag(tag)
																	)
																)
															)
														);
														let message_json = match serde_json::to_string(&message) {
															Ok(msg) => msg,
															Err(error) => {
																let data: &DataSignals = use_context(ctx);
																data.errors.modify().push(ErrorData::new_with_error("Failed to serialize tag deletion message.", error));
																return;
															}
														};

														let send_result = ws.send(Message::Text(message_json)).await;
														if let Err(error) = send_result {
															let data: &DataSignals = use_context(ctx);
															data.errors.modify().push(ErrorData::new_with_error("Failed to send tag deletion message.", error));
														}
													});
												}
											};

											view! {
												ctx,
												"Removing this tag will remove all uses of it from the event log."
												button(type="button", on:click=confirm_delete_handler) { "Yes, delete it!" }
												button(type="button", on:click=cancel_delete_handler) { "No, keep it!" }
											}
										} else {
											view! {
												ctx,
												button(type="button", on:click=start_delete_handler) { "Remove Tag" }
											}
										})
									}
									td {
										form(on:submit=replace_tag_handler) {
											input(
												bind:value=entered_replacement_tag,
												list="tags",
												class={
													if entered_replacement_tag_error.get().is_empty() {
														""
													} else {
														"error"
													}
												},
												title=entered_replacement_tag_error.get()
											)
											button(type="submit") { "Replace Tag" }
										}
									}
								}
							} else {
								view! {
									ctx,
									td {
										(tag_playlist)
									}
								}
							})
						}
					}
				}
			)
		}
		(if *can_edit_signal.get() {
			let entered_tag = create_signal(ctx, String::new());
			let entered_tag_error = create_signal(ctx, String::new());

			let entered_description = create_signal(ctx, String::new());
			let entered_description_error = create_signal(ctx, String::new());

			let new_tag_handler = {
				let event_signal = new_event_signal.clone();
				move |event: WebEvent| {
					event.prevent_default();

					let event_signal = event_signal.clone();

					let name = (*entered_tag.get()).clone();
					if name.is_empty() {
						entered_tag_error.set(String::from("Tag name cannot be empty"));
						return;
					}
					if name.contains(',') {
						entered_tag_error.set(String::from("Tag name cannot contain commas"));
						return;
					}
					if tag_names_index.get().contains_key(&name) {
						entered_tag_error.set(String::from("Tag name cannot be the same as another tag"));
						return;
					}

					let description = (*entered_description.get()).clone();
					if description.is_empty() {
						entered_description_error.set(String::from("Description cannot be empty"));
						return;
					}

					let new_tag = Tag {
						id: String::new(),
						name,
						description,
						playlist: String::new()
					};

					spawn_local_scoped(ctx, async move {
						let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
						let mut ws = ws_context.lock().await;

						let message = FromClientMessage::SubscriptionMessage(
							Box::new(
								SubscriptionTargetUpdate::EventUpdate(
									(*event_signal.get()).clone(),
									Box::new(
										EventSubscriptionUpdate::UpdateTag(new_tag)
									)
								)
							)
						);
						let message_json = match serde_json::to_string(&message) {
							Ok(msg) => msg,
							Err(error) => {
								let data: &DataSignals = use_context(ctx);
								data.errors.modify().push(ErrorData::new_with_error("Failed to serialize new tag message.", error));
								return;
							}
						};

						let send_result = ws.send(Message::Text(message_json)).await;
						if let Err(error) = send_result {
							let data: &DataSignals = use_context(ctx);
							data.errors.modify().push(ErrorData::new_with_error("Failed to send new tag message.", error));
						}

						entered_tag.set(String::new());
						entered_tag_error.set(String::new());
						entered_description.set(String::new());
						entered_description_error.set(String::new());
					});
				}
			};

			view! {
				ctx,
				form(id="manage_tags_add_new_tag", on:submit=new_tag_handler) {
					h1 { "Add New Tag" }
					div(id="manage_tags_add_new_tag_name") {
						input(
							bind:value=entered_tag,
							class=if entered_tag_error.get().is_empty() { "" } else { "error" }
						)
						span(class="input_error") { (entered_tag_error.get()) }
					}
					div(id="manage_tags_add_new_tag_description") {
						input(
							bind:value=entered_description,
							class=if entered_description_error.get().is_empty() { "" } else { "error" }
						)
						span(class="input_error") { (entered_description_error.get()) }
					}
					button(type="Submit") { "Add Tag" }
				}
			}
		} else {
			view! { ctx, }
		})
		(if *user_is_admin_signal.get() {
			let entered_event = create_signal(ctx, String::new());
			let entered_event_error = create_signal(ctx, String::new());

			let copy_event_tags_handler = {
				let event_signal = copy_event_signal.clone();

				move |event: WebEvent| {
					event.prevent_default();

					let event_signal = event_signal.clone();

					let event_name = (*entered_event.get()).clone();
					if event_name.is_empty() {
						entered_event_error.set(String::new());
						return;
					}
					let Some(copy_from_event) = event_names_index.get().get(&event_name).cloned() else {
						entered_event_error.set(String::from("Entered event name must match an event"));
						return;
					};

					entered_event_error.set(String::new());

					spawn_local_scoped(ctx, async move {
						let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
						let mut ws = ws_context.lock().await;

						let message = FromClientMessage::SubscriptionMessage(
							Box::new(
								SubscriptionTargetUpdate::EventUpdate(
									(*event_signal.get()).clone(),
									Box::new(
										EventSubscriptionUpdate::CopyTagsFromEvent(copy_from_event)
									)
								)
							)
						);
						let message_json = match serde_json::to_string(&message) {
							Ok(msg) => msg,
							Err(error) => {
								let data: &DataSignals = use_context(ctx);
								data.errors.modify().push(ErrorData::new_with_error("Failed to serialize tag copy message.", error));
								return;
							}
						};

						let send_result = ws.send(Message::Text(message_json)).await;
						if let Err(error) = send_result {
							let data: &DataSignals = use_context(ctx);
							data.errors.modify().push(ErrorData::new_with_error("Failed to send tag copy message.", error));
						}
						entered_event.set(String::new());
					});
				}
			};

			view! {
				ctx,
				form(id="manage_tags_copy_from_event", on:submit=copy_event_tags_handler) {
					h1 { "Copy Tags from Event" }
					p { "This functionality copies tags from the specified other event to this one." }
					input(
						bind:value=entered_event,
						placeholder="Event name",
						class=if entered_event_error.get().is_empty() { "" } else { "error" },
						list="events"
					)
					button(type="submit") { "Copy Tags" }
					span(class="input_error") { (entered_event_error.get()) }
				}
			}
		} else {
			view! { ctx, }
		})
	}
}

#[component]
pub fn EventLogTagsView<G: Html>(ctx: Scope<'_>, props: EventLogTagsProps) -> View<G> {
	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading tags..." }) {
			EventLogTagsLoadedView(id=props.id)
		}
	}
}
