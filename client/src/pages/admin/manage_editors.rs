use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::admin::{AdminEventEditorUpdate, EditorEventAssociation};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AdminManageEditorsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscriptions_result = {
		let subscriptions = vec![
			SubscriptionType::AdminEventEditors,
			SubscriptionType::AdminEvents,
			SubscriptionType::AdminUsers,
		];
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager.set_subscriptions(subscriptions, &mut ws).await
	};
	if let Err(error) = add_subscriptions_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send event editors subscription message.",
			error,
		));
	}

	let all_events = create_memo(ctx, || (*data.all_events.get()).clone());

	let event_entry = create_signal(ctx, String::new());
	let current_event: &Signal<Option<Event>> = create_signal(ctx, None);
	let event_editors = create_memo(ctx, || {
		data.event_editors.track();
		let event = (*current_event.get()).clone();
		let event = match event {
			Some(event) => event,
			None => return Vec::new(),
		};

		let editors: Vec<UserData> = data
			.event_editors
			.get()
			.iter()
			.filter(|association| association.event.id == event.id)
			.map(|association| association.editor.clone())
			.collect();
		editors
	});

	let event_name_index = create_memo(ctx, || {
		let name_index: HashMap<String, Event> = data
			.all_events
			.get()
			.iter()
			.map(|event| (event.name.clone(), event.clone()))
			.collect();
		name_index
	});
	let event_entry_error: &Signal<String> = create_signal(ctx, String::new());
	let non_editor_users_signal = create_memo(ctx, || {
		let event_user_ids: HashSet<String> = event_editors.get().iter().map(|editor| editor.id.clone()).collect();
		let non_editor_users: Vec<UserData> = data
			.all_users
			.get()
			.iter()
			.filter(|editor| !event_user_ids.contains(&editor.id))
			.cloned()
			.collect();
		non_editor_users
	});
	let non_editor_users_name_index = create_memo(ctx, || {
		let non_editor_map: HashMap<String, UserData> = non_editor_users_signal
			.get()
			.iter()
			.map(|user| (user.username.clone(), user.clone()))
			.collect();
		non_editor_map
	});

	let add_user_entry = create_signal(ctx, String::new());
	let add_user_error: &Signal<String> = create_signal(ctx, String::new());

	let event_selection_handler = move |event: WebEvent| {
		event.prevent_default();

		let new_event_name = event_entry.get();
		if new_event_name.is_empty() {
			current_event.set(None);
			event_entry_error.modify().clear();
			return;
		}

		let name_index = event_name_index.get();
		let new_event = name_index.get(&*new_event_name);
		let new_event = match new_event {
			Some(event) => event.clone(),
			None => {
				event_entry_error.set(format!("The event {} doesn't exist.", *new_event_name));
				return;
			}
		};
		event_entry_error.modify().clear();
		current_event.set(Some(new_event));
	};

	let add_user_handler = move |event: WebEvent| {
		event.prevent_default();

		let current_event = (*current_event.get()).clone();
		let current_event = match current_event {
			Some(event) => event,
			None => return,
		};
		let new_editor_name = add_user_entry.get();
		if new_editor_name.is_empty() {
			add_user_error.modify().clear();
			return;
		}

		let name_index = non_editor_users_name_index.get();
		let new_editor = name_index.get(&*new_editor_name);
		let new_editor = match new_editor {
			Some(user) => user.clone(),
			None => {
				add_user_error.set(format!("{} is not a user or is already an editor.", *new_editor_name));
				return;
			}
		};

		add_user_entry.modify().clear();
		add_user_error.modify().clear();

		let association = EditorEventAssociation {
			event: current_event,
			editor: new_editor,
		};
		let message = FromClientMessage::SubscriptionMessage(Box::new(
			SubscriptionTargetUpdate::AdminEventEditorsUpdate(AdminEventEditorUpdate::AddEditor(association)),
		));
		let message_json = match serde_json::to_string(&message) {
			Ok(msg) => msg,
			Err(error) => {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to serialize new editor message.",
					error,
				));
				return;
			}
		};

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors
					.modify()
					.push(ErrorData::new_with_error("Failed to send new editor message.", error));
			}
		});
	};

	view! {
		ctx,
		datalist(id="add_user_selection") {
			Keyed(
				iterable=non_editor_users_signal,
				key=|user| user.id.clone(),
				view=|ctx, user| view! { ctx, option(value=&user.username) }
			)
		}
		datalist(id="events_selection") {
			Keyed(
				iterable=all_events,
				key=|event| event.id.clone(),
				view=|ctx, event| view! { ctx, option(value=&event.name) }
			)
		}

		form(id="admin_event_editors_event_selection", on:submit=event_selection_handler) {
			input(
				list="events_selection",
				placeholder="Event name",
				bind:value=event_entry,
				class=if event_entry_error.get().is_empty() { "" } else { "error" },
				title=*event_entry_error.get()
			)
			button { "Load" }
		}
		(if current_event.get().is_some() {
			view! {
				ctx,
				div(id="admin_event_editors_list") {
					Keyed(
						iterable=event_editors,
						key=|editor| editor.id.clone(),
						view=move |ctx, editor| {
							let remove_click_handler = {
								let editor = editor.clone();
								move |_event: WebEvent| {
									let editor = editor.clone();
									let event = (*current_event.get()).clone().unwrap();
									let association = EditorEventAssociation { event, editor };
									let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminEventEditorsUpdate(AdminEventEditorUpdate::RemoveEditor(association))));
									let message_json = match serde_json::to_string(&message) {
										Ok(msg) => msg,
										Err(error) => {
											let data: &DataSignals = use_context(ctx);
											data.errors.modify().push(ErrorData::new_with_error("Failed to serialize editor removal message.", error));
											return;
										}
									};

									spawn_local_scoped(ctx, async move {
										let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
										let mut ws = ws_context.lock().await;

										if let Err(error) = ws.send(Message::Text(message_json)).await {
											let data: &DataSignals = use_context(ctx);
											data.errors.modify().push(ErrorData::new_with_error("Failed to send editor removal message.", error));
										}
									});
								}
							};
							view! {
								ctx,
								div(class="admin_event_editors_list_name") { (editor.username) }
								div(class="admin_event_editors_list_remove") {
									button(type="button", on:click=remove_click_handler) { "Remove" }
								}
							}
						}
					)
				}
				(if non_editor_users_signal.get().is_empty() {
					view! { ctx, }
				} else {
					view! {
						ctx,
						form(id="admin_event_editors_add_editor", on:submit=add_user_handler) {
							input(
								list="add_user_selection",
								placeholder="Add user as editor",
								bind:value=add_user_entry,
								class=if add_user_error.get().is_empty() { "" } else { "error" },
								title=*add_user_error.get()
							)
							button { "Add" }
						}
					}
				})
			}
		} else {
			view! { ctx, }
		})
	}
}

#[component]
pub fn AdminManageEditorsView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let user_signal: &Signal<Option<UserData>> = use_context(ctx);

	if let Some(user_data) = user_signal.get().as_ref() {
		if !user_data.is_admin {
			spawn_local_scoped(ctx, async {
				navigate("/");
			});
			return view! { ctx, };
		}
	} else {
		spawn_local_scoped(ctx, async {
			navigate("/");
		});
		return view! { ctx, };
	}

	view! {
		ctx,
		Suspense(fallback=view! { ctx, "Loading editors manager..." }) {
			AdminManageEditorsLoadedView
		}
	}
}
