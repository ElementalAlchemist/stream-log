use crate::pages::error::{ErrorData, ErrorView};
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::stream::SplitSink;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use stream_log_shared::messages::admin::{AdminAction, PermissionGroup, PermissionGroupUser};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::{Event as WebEvent, HtmlButtonElement};

#[component]
async fn AssignUsersToGroupsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
	let mut ws = ws_context.lock().await;

	let users_request = RequestMessage::Admin(AdminAction::ListUsers);
	let users_request_json = match serde_json::to_string(&users_request) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to serialize user request",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};
	if let Err(error) = ws.send(Message::Text(users_request_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error("Failed to send user request", error)));
		return view! { ctx, ErrorView };
	}

	let groups_request = RequestMessage::Admin(AdminAction::ListPermissionGroups);
	let groups_request_json = match serde_json::to_string(&groups_request) {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to serialize groups request",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};
	if let Err(error) = ws.send(Message::Text(groups_request_json)).await {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new_with_error("Failed to send groups request", error)));
		return view! { ctx, ErrorView };
	}

	let users_response: DataMessage<Vec<UserData>> = match read_websocket(&mut ws).await {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to receive user response message",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let groups_response: DataMessage<Vec<PermissionGroup>> = match read_websocket(&mut ws).await {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Failed to receive group response message",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let users = match users_response {
		Ok(users) => users,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"A server error occurred getting users",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let groups = match groups_response {
		Ok(groups) => groups,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"A server error occurred getting permission groups",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let users_signal = create_signal(ctx, users);
	let users_name_index_signal = create_memo(ctx, || {
		let name_indexed_users: HashMap<String, UserData> = users_signal
			.get()
			.iter()
			.map(|user| (user.username.clone(), user.clone()))
			.collect();
		name_indexed_users
	});

	let groups_signal = create_signal(ctx, groups);
	let groups_name_index_signal = create_memo(ctx, || {
		let name_indexed_groups: HashMap<String, PermissionGroup> = groups_signal
			.get()
			.iter()
			.map(|group| (group.name.clone(), group.clone()))
			.collect();
		name_indexed_groups
	});

	let selected_user_signal: &Signal<Option<UserData>> = create_signal(ctx, None);
	let entered_user_name_signal = create_signal(ctx, String::new());
	let entered_user_error_signal: &Signal<Option<String>> = create_signal(ctx, None);

	let selected_user_groups_signal: &Signal<Vec<PermissionGroup>> = create_signal(ctx, Vec::new());

	let addable_groups_signal = create_memo(ctx, || {
		if selected_user_signal.get().is_some() {
			let selected_groups_index: HashSet<PermissionGroup> =
				selected_user_groups_signal.get().iter().cloned().collect();
			let remaining_groups: Vec<PermissionGroup> = groups_signal
				.get()
				.iter()
				.filter(|group| !selected_groups_index.contains(group))
				.cloned()
				.collect();
			remaining_groups
		} else {
			Vec::new()
		}
	});

	let entered_group_name_signal = create_signal(ctx, String::new());
	let entered_group_error_signal: &Signal<Option<String>> = create_signal(ctx, None);

	create_effect(ctx, move || {
		let Some(initial_selected_user) = (*selected_user_signal.get()).clone() else {
			selected_user_groups_signal.set(Vec::new());
			return;
		};

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message = RequestMessage::Admin(AdminAction::ListUserPermissionGroups(initial_selected_user.clone()));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize user permission group list request",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send user permission group list request",
					error,
				)));
				navigate("/error");
				return;
			};

			let group_response: DataMessage<Vec<PermissionGroup>> = match read_websocket(&mut ws).await {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to receive user permission group list response",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			let groups = match group_response {
				Ok(groups) => groups,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"A server error occurred generating the user's permission group list",
						error,
					)));
					navigate("/error");
					return;
				}
			};

			let current_selected_user = (*selected_user_signal.get()).clone();
			if let Some(current_user) = current_selected_user {
				if initial_selected_user == current_user {
					selected_user_groups_signal.set(groups);
				}
			}
		});
	});

	let user_submission_handler = |event: WebEvent| {
		event.prevent_default();

		let entered_name = entered_user_name_signal.get();
		if entered_name.is_empty() {
			selected_user_signal.set(None);
			return;
		}

		let name_index = users_name_index_signal.get();
		let user = if let Some(user) = name_index.get(entered_name.as_ref()) {
			user.clone()
		} else {
			entered_user_error_signal.set(Some(String::from("That name doesn't match any users.")));
			return;
		};

		selected_user_signal.set(Some(user));
	};

	let clear_user_error_handler = |_event: WebEvent| {
		entered_user_error_signal.set(None);
	};

	let add_group_submission_handler = move |event: WebEvent| {
		event.prevent_default();

		let entered_group_name = (*entered_group_name_signal.get()).clone();
		if entered_group_name.is_empty() {
			return;
		}

		let Some(group) = groups_name_index_signal.get().get(&entered_group_name).cloned() else {
			entered_group_error_signal.set(Some(String::from("That name doesn't match any groups.")));
			return;
		};

		entered_group_name_signal.set(String::new());

		let Some(user) = (*selected_user_signal.get()).clone() else {
			return;
		};

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let permission_group_user = PermissionGroupUser {
				group: group.clone(),
				user: user.clone(),
			};
			let message = RequestMessage::Admin(AdminAction::AddUserToPermissionGroup(permission_group_user));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						"Failed to serialize request to add user to group",
						error,
					)));
					navigate("/error");
					return;
				}
			};
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					"Failed to send request to add user to group",
					error,
				)));
				navigate("/error");
				return;
			}
			if let Some(current_user) = (*selected_user_signal.get()).clone() {
				// Verify the user didn't change across the various awaits
				if user != current_user {
					return;
				}

				selected_user_groups_signal.modify().push(group);
			}
		});
	};

	let clear_group_error_handler = |_event: WebEvent| {
		entered_group_error_signal.set(None);
	};

	view! {
		ctx,
		datalist(id="all_users") {
			Keyed(
				iterable=users_signal,
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
				on:change=clear_user_error_handler,
				class=(if entered_user_error_signal.get().is_some() { "error" } else { "" })
			)
			(if let Some(error_msg) = (*entered_user_error_signal.get()).clone() {
				view! { ctx, span(class="input_error") { (error_msg) } }
			} else {
				view! { ctx, }
			})
		}
		div(id="admin_assign_groups_list_for_user") {
			Keyed(
				iterable=selected_user_groups_signal,
				key=|group| group.id.clone(),
				view=move |ctx, group| {
					let remove_button_ref = create_node_ref(ctx);
					let remove_handler = {
						let group = group.clone();
						move |_event: WebEvent| {
							let remove_button_node: DomNode = remove_button_ref.get();
							let remove_button: HtmlButtonElement = remove_button_node.unchecked_into();
							remove_button.set_disabled(true);

							spawn_local_scoped(ctx, {
								let group = group.clone();
								async move {
									let user = if let Some(user) = selected_user_signal.get().as_ref() {
										user.clone()
									} else {
										return;
									};
									let user_permission_group = PermissionGroupUser { user: user.clone(), group: group.clone() };
									let message = RequestMessage::Admin(AdminAction::RemoveUserFromPermissionGroup(user_permission_group));
									let message_json = match serde_json::to_string(&message) {
										Ok(msg) => msg,
										Err(error) => {
											let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
											error_signal.set(Some(ErrorData::new_with_error("Failed to serialize permission group removal request", error)));
											navigate("/error");
											return;
										}
									};

									let ws_context: &Mutex<SplitSink<WebSocket, Message>> = use_context(ctx);
									let mut ws = ws_context.lock().await;

									if let Err(error) = ws.send(Message::Text(message_json)).await {
										let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
										error_signal.set(Some(ErrorData::new_with_error("Failed to send permission group removal request", error)));
										navigate("/error");
										return;
									}

									// We want to ensure that the user didn't change while we were sending the request.
									if let Some(selected_user) = selected_user_signal.get().as_ref() {
										if user != *selected_user {
											return;
										}
									} else {
										return;
									}

									let mut modify_groups = selected_user_groups_signal.modify();
									if let Some((index, _)) = modify_groups.iter().enumerate().find(|(_, g)| g.id == group.id) {
										modify_groups.remove(index);
									}
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
								button(ref=remove_button_ref, on:click=remove_handler) {
									"Remove"
								}
							}
						}
					}
				}
			)
		}
		(if !addable_groups_signal.get().is_empty() {
			view! {
				ctx,
				form(id="admin_assign_groups_add_to_user", on:submit=add_group_submission_handler) {
					input(
						id="admin_assign_add_group",
						placeholder="Add group for user...",
						list="addable_groups",
						bind:value=entered_group_name_signal,
						on:change=clear_group_error_handler,
						class=(if entered_group_error_signal.get().is_some() { "error" } else { "" })
					)
					(if let Some(error_msg) = (*entered_group_error_signal.get()).clone() {
						view! { ctx, span(class="input_error") { (error_msg) } }
					} else {
						view! { ctx, }
					})
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
