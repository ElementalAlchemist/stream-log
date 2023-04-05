use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::DataSignals;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::admin::{
	AdminUserPermissionGroupUpdate, PermissionGroup, UserPermissionGroupAssociation,
};
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AssignUsersToGroupsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;
	let data: &DataSignals = use_context(ctx);

	let subscription_message =
		FromClientMessage::StartSubscription(SubscriptionType::AdminUserPermissionGroupAssignment);
	let subscription_message_json = match serde_json::to_string(&subscription_message) {
		Ok(msg) => msg,
		Err(error) => {
			data.errors.modify().push(ErrorData::new_with_error(
				"Failed to serialize user permission group assignment subscription message.",
				error,
			));
			return view! { ctx, };
		}
	};

	if let Err(error) = ws.send(Message::Text(subscription_message_json)).await {
		data.errors.modify().push(ErrorData::new_with_error(
			"Failed to send user permission group assignment subscription message.",
			error,
		));
	}

	let users_name_index_signal = create_memo(ctx, || {
		let name_indexed_users: HashMap<String, UserData> = data
			.all_users
			.get()
			.iter()
			.map(|user| (user.username.clone(), user.clone()))
			.collect();
		name_indexed_users
	});

	let groups_name_index_signal = create_memo(ctx, || {
		let name_indexed_groups: HashMap<String, PermissionGroup> = data
			.all_permission_groups
			.get()
			.iter()
			.map(|group| (group.name.clone(), group.clone()))
			.collect();
		name_indexed_groups
	});

	let selected_user_signal: &Signal<Option<UserData>> = create_signal(ctx, None);
	let entered_user_name_signal = create_signal(ctx, String::new());
	let entered_user_error_signal: &Signal<String> = create_signal(ctx, String::new());

	let user_groups_signal = create_memo(ctx, || {
		data.user_permission_groups.track();
		match selected_user_signal.get().as_ref() {
			Some(user) => {
				let groups: Vec<PermissionGroup> = data
					.user_permission_groups
					.get()
					.iter()
					.filter(|association| association.user == *user)
					.map(|association| association.permission_group.clone())
					.collect();
				groups
			}
			None => Vec::new(),
		}
	});

	let addable_groups_signal = create_memo(ctx, || {
		user_groups_signal.track();
		let selected_user = match selected_user_signal.get().as_ref() {
			Some(user) => user.clone(),
			None => return Vec::new(),
		};
		let user_groups: HashSet<PermissionGroup> = user_groups_signal.get().iter().cloned().collect();
		let groups: Vec<PermissionGroup> = data
			.all_permission_groups
			.get()
			.iter()
			.filter(|group| !user_groups.contains(*group))
			.cloned()
			.collect();
		groups
	});
	let has_addable_groups_signal = create_memo(ctx, || !addable_groups_signal.get().is_empty());

	let entered_group_name_signal = create_signal(ctx, String::new());
	let entered_group_error_signal: &Signal<String> = create_signal(ctx, String::new());

	let user_submission_handler = |event: WebEvent| {
		event.prevent_default();

		let entered_name = entered_user_name_signal.get();
		if entered_name.is_empty() {
			selected_user_signal.set(None);
			entered_user_error_signal.modify().clear();
			return;
		}

		let name_index = users_name_index_signal.get();
		let user = if let Some(user) = name_index.get(entered_name.as_ref()) {
			user.clone()
		} else {
			entered_user_error_signal.set(String::from("That name doesn't match any users."));
			return;
		};

		selected_user_signal.set(Some(user));
		entered_user_error_signal.modify().clear();
	};

	let add_group_submission_handler = move |event: WebEvent| {
		event.prevent_default();

		let entered_group_name = (*entered_group_name_signal.get()).clone();
		if entered_group_name.is_empty() {
			entered_group_error_signal.modify().clear();
			return;
		}

		let Some(permission_group) = groups_name_index_signal.get().get(&entered_group_name).cloned() else {
			entered_group_error_signal.set(String::from("That name doesn't match any groups."));
			return;
		};

		entered_group_name_signal.set(String::new());
		entered_group_error_signal.modify().clear();

		let Some(user) = (*selected_user_signal.get()).clone() else {
			return;
		};

		let permission_group_user = UserPermissionGroupAssociation { user, permission_group };
		let message = FromClientMessage::SubscriptionMessage(Box::new(
			SubscriptionTargetUpdate::AdminUserPermissionGroupsUpdate(AdminUserPermissionGroupUpdate::AddUserToGroup(
				permission_group_user,
			)),
		));
		let message_json = match serde_json::to_string(&message) {
			Ok(msg) => msg,
			Err(error) => {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to serialize user group addition message.",
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
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to send user group addition message.",
					error,
				));
			}
		});
	};

	view! {
		ctx,
		datalist(id="all_users") {
			Keyed(
				iterable=data.all_users,
				key=|user| user.id.clone(),
				view=|ctx, user| {
					view! {
						ctx,
						option(value=user.username)
					}
				}
			)
		}
		datalist(id="addable_groups") {
			Keyed(
				iterable=addable_groups_signal,
				key=|group| group.id.clone(),
				view=|ctx, group| {
					view! {
						ctx,
						option(value=group.name)
					}
				}
			)
		}
		form(on:submit=user_submission_handler) {
			input(
				id="admin_assign_user",
				list="all_users",
				placeholder="Enter a user...",
				bind:value=entered_user_name_signal,
				class=if entered_user_error_signal.get().is_empty() { "" } else { "error" },
				title=*entered_user_error_signal.get()
			)
			button(type="submit") { "Load User Groups" }
		}
		div(id="admin_assign_groups_list_for_user") {
			Keyed(
				iterable=user_groups_signal,
				key=|group| group.id.clone(),
				view=move |ctx, group| {
					let remove_handler = {
						let group = group.clone();
						move |_event: WebEvent| {
							let group = group.clone();

							let user = if let Some(user) = selected_user_signal.get().as_ref() {
								user.clone()
							} else {
								return;
							};
							let user_permission_group = UserPermissionGroupAssociation { user: user.clone(), permission_group: group.clone() };
							let message = FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminUserPermissionGroupsUpdate(AdminUserPermissionGroupUpdate::RemoveUserFromGroup(user_permission_group))));
							let message_json = match serde_json::to_string(&message) {
								Ok(msg) => msg,
								Err(error) => {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to serialize permission group user removal message.", error));
									return;
								}
							};

							spawn_local_scoped(ctx, async move {
								let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								if let Err(error) = ws.send(Message::Text(message_json)).await {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to send permission group user removal message.", error));
								}
							});
						}
					};

					view! {
						ctx,
						div(class="admin_assign_group_row") {
							div(class="admin_assign_group_name") {
								(group.name)
							}
							div(class="admin_assign_group_remove") {
								button(on:click=remove_handler) {
									"Remove"
								}
							}
						}
					}
				}
			)
		}
		(if !*has_addable_groups_signal.get() {
			view! {
				ctx,
				form(id="admin_assign_groups_add_to_user", on:submit=add_group_submission_handler) {
					input(
						id="admin_assign_add_group",
						placeholder="Add group for user...",
						list="addable_groups",
						bind:value=entered_group_name_signal,
						class=if entered_group_error_signal.get().is_empty() { "" } else { "error" },
						title=*entered_group_error_signal.get()
					)
					button(type="submit") { "Add Group" }
				}
			}
		} else {
			view! { ctx, }
		})
		button(id="admin_assign_user_groups_done", on:click=|_| navigate("/")) { "Done" }
	}
}

#[component]
pub fn AssignUsersToGroupsView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let user: &Signal<Option<UserData>> = use_context(ctx);
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
