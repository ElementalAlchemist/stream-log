// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::color_utils::rgb_str_from_color;
use crate::page_utils::set_page_title;
use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use crate::websocket::WebSocketSendStream;
use futures::lock::Mutex;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::admin::{
	AdminUserPermissionGroupUpdate, PermissionGroup, UserPermissionGroupAssociation,
};
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::SelfUserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AssignUsersToGroupsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	set_page_title("Assign Users to Permission Groups | Stream Log");

	let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let add_subscriptions_result = {
		let subscriptions = vec![
			SubscriptionType::AdminUsers,
			SubscriptionType::AdminPermissionGroups,
			SubscriptionType::AdminPermissionGroupUsers,
		];
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager.set_subscriptions(subscriptions, &mut ws).await
	};
	if let Err(error) = add_subscriptions_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Couldn't send user permission group assignment subscriptoin message.",
			error,
		));
	}

	let all_users = create_memo(ctx, || (*data.all_users.get()).clone());
	let all_groups = create_memo(ctx, || (*data.all_permission_groups.get()).clone());

	let groups_name_index_signal = create_memo(ctx, || {
		let name_indexed_groups: HashMap<String, PermissionGroup> = data
			.all_permission_groups
			.get()
			.iter()
			.map(|group| (group.name.clone(), group.clone()))
			.collect();
		name_indexed_groups
	});

	let selected_group_signal: &Signal<Option<PermissionGroup>> = create_signal(ctx, None);
	let entered_group_name_signal = create_signal(ctx, String::new());
	let entered_group_error_signal = create_signal(ctx, String::new());

	let group_users_signal = create_memo(ctx, || {
		data.user_permission_groups.track();
		match selected_group_signal.get().as_ref() {
			Some(group) => {
				let user_ids: HashSet<String> = data
					.user_permission_groups
					.get()
					.iter()
					.filter(|association| association.permission_group.id == group.id)
					.map(|association| association.user.id.clone())
					.collect();
				user_ids
			}
			None => HashSet::new(),
		}
	});

	let group_selection_handler = |event: WebEvent| {
		event.prevent_default();

		let group_name = entered_group_name_signal.get();
		if group_name.is_empty() {
			entered_group_error_signal.set(String::new());
			selected_group_signal.set(None);
			return;
		}
		let groups_name_index = groups_name_index_signal.get();
		let Some(group) = groups_name_index.get(&*group_name) else {
			entered_group_error_signal.set(String::from("That's not the name of a group."));
			return;
		};

		selected_group_signal.set(Some(group.clone()));
		entered_group_error_signal.set(String::new());
	};

	view! {
		ctx,
		datalist(id="all_groups") {
			Keyed(
				iterable=all_groups,
				key=|group| group.id.clone(),
				view=|ctx, group| {
					view! {
						ctx,
						option(value=group.name)
					}
				}
			)
		}
		form(on:submit=group_selection_handler) {
			input(
				id="admin_assign_group",
				list="all_groups",
				placeholder="Enter a group...",
				bind:value=entered_group_name_signal,
				class=if entered_group_error_signal.get().is_empty() { "" } else { "error" }
			)
			button(type="submit") { "Load User Groups" }
			span(class="input_error") { (*entered_group_error_signal.get()) }
		}
		table(id="admin_assign_groups_user_list") {
			(if selected_group_signal.get().is_some() {
				view! {
					ctx,
					Keyed(
						iterable=all_users,
						key=|user| user.id.clone(),
						view=move |ctx, user| {
							let user_in_group = create_memo(ctx, {
								let user_id = user.id.clone();
								move || group_users_signal.get().contains(&user_id)
							});
							let change_user_group_handler = {
								let user = user.clone();
								move |_event: WebEvent| {
									let user = user.clone();
									let group = match selected_group_signal.get().as_ref() {
										Some(group) => group.clone(),
										None => return
									};
									spawn_local_scoped(ctx, async move {
										let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
										let mut ws = ws_context.lock().await;

										let user_group_association = UserPermissionGroupAssociation { user: user.clone().into(), permission_group: group };
										let user_group_message = if *user_in_group.get() {
											AdminUserPermissionGroupUpdate::RemoveUserFromGroup(user_group_association)
										} else {
											AdminUserPermissionGroupUpdate::AddUserToGroup(user_group_association)
										};
										let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminUserPermissionGroupsUpdate(user_group_message)));
										let message_json = match serde_json::to_string(&message) {
											Ok(msg) => msg,
											Err(error) => {
												let data: &DataSignals = use_context(ctx);
												data.errors.modify().push(ErrorData::new_with_error("Failed to serialize user permission group update.", error));
												return;
											}
										};

										let send_result = ws.send(Message::Text(message_json)).await;
										if let Err(error) = send_result {
											let data: &DataSignals = use_context(ctx);
											data.errors.modify().push(ErrorData::new_with_error("Failed to send user permission group update.", error));
										}
									});
								}
							};

							let username_style = format!("color: {}", rgb_str_from_color(user.color));

							view! {
								ctx,
								tr {
									td(style=username_style) { (user.username) }
									td {
										(if *user_in_group.get() {
											"✔️"
										} else {
											""
										})
									}
									td {
										button(type="button", on:click=change_user_group_handler) {
											(if *user_in_group.get() {
												"Remove"
											} else {
												"Add"
											})
										}
									}
								}
							}
						}
					)
				}
			} else {
				view! { ctx, }
			})
		}
	}
}

#[component]
pub fn AssignUsersToGroupsView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let user: &Signal<Option<SelfUserData>> = use_context(ctx);
	match user.get().as_ref() {
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
		Suspense(fallback=view!{ ctx, "Loading assignment data..." }) {
			AssignUsersToGroupsLoadedView
		}
	}
}
