use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::HashMap;
use stream_log_shared::messages::admin::{
	AdminPermissionGroupUpdate, PermissionGroup, PermissionGroupEventAssociation,
};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AdminManageGroupsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscription_result = {
		let subscriptions = vec![SubscriptionType::AdminEvents, SubscriptionType::AdminPermissionGroups];
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager.set_subscriptions(subscriptions, &mut ws).await
	};
	if let Err(error) = add_subscription_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send permission group subscription message.",
			error,
		));
	}

	let all_permission_groups = create_memo(ctx, || (*data.all_permission_groups.get()).clone());
	let all_events = create_memo(ctx, || (*data.all_events.get()).clone());
	let permission_group_events = create_memo(ctx, || {
		let mut group_events: HashMap<String, HashMap<String, PermissionLevel>> = HashMap::new();
		for group_event_association in data.permission_group_event_associations.get().iter() {
			let group_id = group_event_association.group.clone();
			let event_id = group_event_association.event.clone();
			group_events
				.entry(group_id)
				.or_default()
				.insert(event_id, group_event_association.permission);
		}
		group_events
	});

	let event_names_index_signal = create_memo(ctx, || {
		let event_names: HashMap<String, Event> = data
			.all_events
			.get()
			.iter()
			.map(|event| (event.name.clone(), event.clone()))
			.collect();
		event_names
	});

	let new_group_name_signal = create_signal(ctx, String::new());
	let new_group_error_signal = create_signal(ctx, String::new());
	let new_group_submit_handler = move |event: WebEvent| {
		event.prevent_default();

		let new_group_name = (*new_group_name_signal.get()).clone();
		if event_names_index_signal.get().contains_key(&new_group_name) {
			new_group_error_signal.set(format!("The group \"{}\" already exists.", new_group_name));
			return;
		}
		new_group_error_signal.modify().clear();
		new_group_name_signal.set(String::new());

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let new_group = PermissionGroup {
				id: String::new(),
				name: new_group_name,
			};
			let message = FromClientMessage::SubscriptionMessage(Box::new(
				SubscriptionTargetUpdate::AdminPermissionGroupsUpdate(AdminPermissionGroupUpdate::UpdateGroup(
					new_group,
				)),
			));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize permission group creation message.",
						error,
					));
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to send permission group creation message.",
					error,
				));
			}
		});
	};

	view! {
		ctx,
		div(id="admin_manage_groups") {
			h1 { "Permission Groups" }
			Keyed(
				iterable=all_permission_groups,
				key=|group| group.id.clone(),
				view=move |ctx, group| {
					let group_id = group.id.clone();
					let event_permissions = create_memo(ctx, move || {
						permission_group_events.get().get(&group_id).cloned().unwrap_or_default()
					});

					let group_name_signal = create_signal(ctx, group.name.clone());

					let submit_group_name_handler = {
						let group = group.clone();
						move |event: WebEvent| {
							event.prevent_default();
							let group = group.clone();

							spawn_local_scoped(ctx, async move {
								let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								let mut new_group = group;
								new_group.name = (*group_name_signal.get()).clone();
								let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminPermissionGroupsUpdate(AdminPermissionGroupUpdate::UpdateGroup(new_group))));
								let message_json = match serde_json::to_string(&message) {
									Ok(msg) => msg,
									Err(error) => {
										let data: &DataSignals = use_context(ctx);
										data.errors.modify().push(ErrorData::new_with_error("Failed to serialize permission group update message.", error));
										return;
									}
								};
								if let Err(error) = ws.send(Message::Text(message_json)).await {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to send permission group update message.", error));
								}
							});
						}
					};

					view! {
						ctx,
						div(class="admin_manage_groups_name") {
							form(class="admin_manage_groups_group", on:submit=submit_group_name_handler) {
								input(bind:value=group_name_signal)
								button(type="submit") { "Update Name" }
							}
						}
						div(class="admin_manage_groups_events") {
							div(class="admin_manage_groups_events_header") { "Event" }
							div(class="admin_manage_groups_events_header") { "View" }
							div(class="admin_manage_groups_events_header") { "Edit" }
							div(class="admin_manage_groups_events_header") { "Supervisor" }
							div(class="admin_manage_groups_events_header") { }

							Keyed(
								iterable=all_events,
								key=|event| event.id.clone(),
								view=move |ctx, event| {
									let group = group.clone();
									let can_view_signal = create_signal(ctx, false);
									let can_edit_signal = create_signal(ctx, false);
									let is_supervisor_signal = create_signal(ctx, false);

									create_effect(ctx, {
										let event_id = event.id.clone();
										move || {
											let event_permissions_data = event_permissions.get();
											let permission = event_permissions_data.get(&event_id);
											let (can_view, can_edit, is_supervisor) = match permission {
												Some(PermissionLevel::Supervisor) => (true, true, true),
												Some(PermissionLevel::Edit) => (true, true, false),
												Some(PermissionLevel::View) => (true, false, false),
												None => (false, false, false)
											};
											can_view_signal.set(can_view);
											can_edit_signal.set(can_edit);
											is_supervisor_signal.set(is_supervisor);
										}
									});

									create_effect(ctx, || {
										if *is_supervisor_signal.get() {
											can_edit_signal.set(true);
											can_view_signal.set(true);
										}
									});
									create_effect(ctx, || {
										if *can_edit_signal.get() {
											can_view_signal.set(true);
										} else {
											is_supervisor_signal.set(false);
										}
									});
									create_effect(ctx, || {
										if !*can_view_signal.get() {
											can_edit_signal.set(false);
											is_supervisor_signal.set(false);
										}
									});

									let update_handler = {
										let event = event.clone();
										move |web_event: WebEvent| {
											web_event.prevent_default();

											let message = if *is_supervisor_signal.get() {
												AdminPermissionGroupUpdate::SetEventPermissionForGroup(PermissionGroupEventAssociation { group: group.id.clone(), event: event.id.clone(), permission: PermissionLevel::Supervisor })
											} else if *can_edit_signal.get() {
												AdminPermissionGroupUpdate::SetEventPermissionForGroup(PermissionGroupEventAssociation { group: group.id.clone(), event: event.id.clone(), permission: PermissionLevel::Edit })
											} else if *can_view_signal.get() {
												AdminPermissionGroupUpdate::SetEventPermissionForGroup(PermissionGroupEventAssociation { group: group.id.clone(), event: event.id.clone(), permission: PermissionLevel::View })
											} else {
												AdminPermissionGroupUpdate::RemoveEventFromGroup(group.clone(), event.clone())
											};

											spawn_local_scoped(ctx, async move {
												let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
												let mut ws = ws_context.lock().await;

												let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminPermissionGroupsUpdate(message)));
												let message_json = match serde_json::to_string(&message) {
													Ok(msg) => msg,
													Err(error) => {
														let data: &DataSignals = use_context(ctx);
														data.errors.modify().push(ErrorData::new_with_error("Failed to serialize permission update for permission group.", error));
														return;
													}
												};
												if let Err(error) = ws.send(Message::Text(message_json)).await {
													let data: &DataSignals = use_context(ctx);
													data.errors.modify().push(ErrorData::new_with_error("Failed to send permission update for permission group.", error));
												}
											});
										}
									};

									view! {
										ctx,
										form(class="admin_manage_groups_events_row", on:submit=update_handler) {
											div(class="admin_manage_groups_events_name") { (event.name) }
											div(class="admin_manage_groups_events_view") {
												input(type="checkbox", bind:checked=can_view_signal)
											}
											div(class="admin_manage_groups_events_edit") {
												input(type="checkbox", bind:checked=can_edit_signal)
											}
											div(class="admin_manage_gropus_events_supervisor") {
												input(type="checkbox", bind:checked=is_supervisor_signal)
											}
											div(class="admin_manage_groups_events_update") {
												button(type="submit") { "Update" }
											}
										}
									}
								}
							)
						}
					}
				}
			)
			form(id="admin_manage_groups_new_group", on:submit=new_group_submit_handler) {
				input(bind:value=new_group_name_signal, placeholder="New group name", class=if new_group_error_signal.get().is_empty() { "" } else { "error" })
				button(type="submit") { "Add group" }
				span(id="admin_manage_groups_new_group_error") { (new_group_error_signal.get()) }
			}
		}
	}
}

#[component]
pub fn AdminManageGroupsView<G: Html>(ctx: Scope<'_>) -> View<G> {
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
		Suspense(
			fallback=view! { ctx, "Loading permission groups..." }
		) {
			AdminManageGroupsLoadedView
		}
	}
}
