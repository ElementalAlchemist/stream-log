use crate::pages::error::{ErrorData, ErrorView};
use crate::websocket::read_websocket;
use futures::lock::Mutex;
use futures::SinkExt;
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use stream_log_shared::messages::admin::{AdminAction, EventPermission, PermissionGroup, PermissionGroupWithEvents};
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::permissions::PermissionLevel;
use stream_log_shared::messages::{DataMessage, RequestMessage};
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::{Event as WebEvent, HtmlButtonElement, HtmlInputElement, HtmlSpanElement};

#[derive(Clone, Eq, PartialEq)]
struct OptionalEventPermission {
	event: Event,
	level: Option<PermissionLevel>,
}

impl From<EventPermission> for OptionalEventPermission {
	fn from(event_permission: EventPermission) -> Self {
		Self {
			event: event_permission.event,
			level: Some(event_permission.level),
		}
	}
}

#[derive(Clone, Eq, PartialEq)]
struct PermissionGroupWithOptionalEvents {
	group: PermissionGroup,
	events: Vec<OptionalEventPermission>,
}

impl From<PermissionGroupWithEvents> for PermissionGroupWithOptionalEvents {
	fn from(mut group_data: PermissionGroupWithEvents) -> Self {
		Self {
			group: group_data.group,
			events: group_data.events.drain(..).map(|event| event.into()).collect(),
		}
	}
}

impl From<PermissionGroupWithOptionalEvents> for PermissionGroupWithEvents {
	fn from(mut group_data: PermissionGroupWithOptionalEvents) -> Self {
		Self {
			group: group_data.group,
			events: group_data
				.events
				.drain(..)
				.filter_map(|event| {
					Some(EventPermission {
						event: event.event,
						level: event.level?,
					})
				})
				.collect(),
		}
	}
}

#[component]
async fn AdminManageGroupsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let (events, mut permission_groups) = {
		let ws_context: &Mutex<WebSocket> = use_context(ctx);
		let mut ws = ws_context.lock().await;

		let message = RequestMessage::Admin(AdminAction::ListEvents);
		let message_json = match serde_json::to_string(&message) {
			Ok(msg) => msg,
			Err(error) => {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("Failed to serialize events request"),
					error,
				)));
				return view! { ctx, ErrorView };
			}
		};
		if let Err(error) = ws.send(Message::Text(message_json)).await {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to send events request"),
				error,
			)));
			return view! { ctx, ErrorView };
		}

		let message = RequestMessage::Admin(AdminAction::ListPermissionGroups);
		let message_json = match serde_json::to_string(&message) {
			Ok(msg) => msg,
			Err(error) => {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("Failed to serialize permission groups request"),
					error,
				)));
				return view! { ctx, ErrorView };
			}
		};
		if let Err(error) = ws.send(Message::Text(message_json)).await {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				String::from("Failed to send permission groups request"),
				error,
			)));
			return view! { ctx, ErrorView };
		}

		let events: DataMessage<Vec<Event>> = match read_websocket(&mut ws).await {
			Ok(data) => data,
			Err(error) => {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("Failed to receive events data"),
					error,
				)));
				return view! { ctx, ErrorView };
			}
		};

		let permission_groups: DataMessage<Vec<PermissionGroupWithEvents>> = match read_websocket(&mut ws).await {
			Ok(data) => data,
			Err(error) => {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("Failed to receive permission groups data"),
					error,
				)));
				return view! { ctx, ErrorView };
			}
		};

		let events = match events {
			Ok(events) => events,
			Err(error) => {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("Server error occurred getting events data"),
					error,
				)));
				return view! { ctx, ErrorView };
			}
		};

		let permission_groups = match permission_groups {
			Ok(groups) => groups,
			Err(error) => {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("Server error occurred getting permission group data"),
					error,
				)));
				return view! { ctx, ErrorView };
			}
		};

		(events, permission_groups)
	};

	let events_signal = create_signal(ctx, events);
	let events_names_index_signal = create_memo(ctx, || {
		let events_names_index: HashMap<String, Event> = events_signal
			.get()
			.iter()
			.map(|event| (event.name.clone(), event.clone()))
			.collect();
		events_names_index
	});

	let permission_groups: Vec<PermissionGroupWithOptionalEvents> = permission_groups
		.drain(..)
		.map(|group_data| group_data.into())
		.collect();
	let permission_groups_signal = create_signal(ctx, permission_groups);
	let updated_groups_signal: &Signal<HashSet<String>> = create_signal(ctx, HashSet::new());
	let expanded_group: &Signal<Option<String>> = create_signal(ctx, None);

	let submit_button = create_node_ref(ctx);
	let cancel_button = create_node_ref(ctx);

	let next_new_group_id = Rc::new(AtomicU32::new(0));

	let form_submission_handler = move |event: WebEvent| {
		event.prevent_default();

		let submit_button_ref: DomNode = submit_button.get();
		let submit_button: HtmlButtonElement = submit_button_ref.unchecked_into();
		let cancel_button_ref: DomNode = cancel_button.get();
		let cancel_button: HtmlButtonElement = cancel_button_ref.unchecked_into();

		submit_button.set_disabled(true);
		cancel_button.set_disabled(true);

		let updated_groups = updated_groups_signal.get();
		let mut submit_groups: Vec<PermissionGroupWithEvents> = permission_groups_signal
			.get()
			.iter()
			.filter(|group_data| updated_groups.contains(&group_data.group.id))
			.cloned()
			.map(|group_data| group_data.into())
			.collect();
		for group_data in submit_groups.iter_mut() {
			if !group_data.group.id.starts_with('+') && group_data.group.name.is_empty() {
				// Field validation should take care of error visibility already
				submit_button.set_disabled(false);
				cancel_button.set_disabled(false);
				return;
			}
			if group_data.group.id.starts_with('+') {
				group_data.group.id = String::new();
			}
		}

		spawn_local_scoped(ctx, async move {
			let message = RequestMessage::Admin(AdminAction::UpdatePermissionGroups(submit_groups));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
					error_signal.set(Some(ErrorData::new_with_error(
						String::from("Failed to serialize permission group update"),
						error,
					)));
					navigate("/error");
					return;
				}
			};

			let ws_context: &Mutex<WebSocket> = use_context(ctx);
			let mut ws = ws_context.lock().await;
			if let Err(error) = ws.send(Message::Text(message_json)).await {
				let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
				error_signal.set(Some(ErrorData::new_with_error(
					String::from("failed to send permission group update"),
					error,
				)));
				navigate("/error");
				return;
			}

			navigate("/");
		});
	};

	let cancel_button_handler = |_event: WebEvent| {
		let submit_button_ref: DomNode = submit_button.get();
		let submit_button: HtmlButtonElement = submit_button_ref.unchecked_into();
		let cancel_button_ref: DomNode = cancel_button.get();
		let cancel_button: HtmlButtonElement = cancel_button_ref.unchecked_into();

		submit_button.set_disabled(true);
		cancel_button.set_disabled(true);

		navigate("/");
	};

	let add_new_button_handler = {
		let next_new_group_id = Rc::clone(&next_new_group_id);
		move |_event: WebEvent| {
			let group_index_num = next_new_group_id.fetch_add(1, Ordering::AcqRel);
			let group = PermissionGroup {
				id: format!("+{}", group_index_num),
				name: String::new(),
			};
			let new_permission_set = PermissionGroupWithOptionalEvents {
				group,
				events: Vec::new(),
			};
			permission_groups_signal.modify().push(new_permission_set);
		}
	};

	view! {
		ctx,
		h1 { "Manage Permission Groups" }
		form(on:submit=form_submission_handler) {
			div {
				Keyed(
					iterable=permission_groups_signal,
					key=|group_with_events| group_with_events.group.id.clone(),
					view=move |ctx, group_data| {
						let group_id = group_data.group.id.clone();
						let this_group_is_open = create_memo(ctx, {
							let group_id = group_id.clone();
							move || (*expanded_group.get()).as_ref().map(|g| group_id == *g).unwrap_or(false)
						});
						let events_class = create_memo(ctx, || {
							if *this_group_is_open.get() {
								"admin_group_events admin_group_events_active"
							} else {
								"admin_group_events"
							}
						});

						let group_name_signal = create_signal(ctx, group_data.group.name.clone());
						let group_name_field = create_node_ref(ctx);

						if !group_id.starts_with('+') {
							create_effect(ctx, || {
								let name_empty = group_name_signal.get().is_empty();

								let group_name_field_ref: DomNode = match group_name_field.try_get() {
									Some(node_ref) => node_ref,
									None => return
								};
								let group_name_field: HtmlInputElement = group_name_field_ref.unchecked_into();
								if name_empty {
									group_name_field.class_list().add_1("input-error").expect("Class change is valid");
								} else {
									group_name_field.class_list().remove_1("input-error").expect("Class change is valid");
								}
							});
						}
						create_effect(ctx, {
							let group_id = group_id.clone();
							move || {
								let new_name = (*group_name_signal.get()).clone();
								let mut permission_groups_modification = permission_groups_signal.modify();
								let group_data = permission_groups_modification.iter_mut().find(|g| g.group.id == group_id).expect("Permission group rendering exists as a permission group");
								group_data.group.name = new_name;
								updated_groups_signal.modify().insert(group_id.clone());
							}
						});

						let group_events_signal = create_signal(ctx, group_data.events);

						create_effect(ctx, {
							let group_id = group_id.clone();
							move || {
								let mut modify_permission_groups = permission_groups_signal.modify();
								let group_data = modify_permission_groups.iter_mut().find(|pg| pg.group.id == group_id).expect("Rendered group exists in the group data");
								group_data.events = (*group_events_signal.get()).clone();
								updated_groups_signal.modify().insert(group_id.clone());
							}
						});

						let available_events_signal: &ReadSignal<Vec<Event>> = create_memo(ctx, || {
							(*events_signal.get()).iter().filter(|ev| !group_events_signal.get().iter().any(|ge| ev.id == ge.event.id)).cloned().collect()
						});

						let group_header_click_handler = {
							let group_id = group_id.clone();
							move |_event: WebEvent| {
								if *this_group_is_open.get() {
									expanded_group.set(None);
								} else {
									expanded_group.set(Some(group_id.clone()));
								}
							}
						};

						let form_error_node = create_node_ref(ctx);
						let new_event_name_signal = create_signal(ctx, String::new());

						create_effect(ctx, || {
							let _ = *new_event_name_signal.get(); // We don't need the value, but this should trigger when name changes
							let error_node_ref: DomNode = match form_error_node.try_get() {
								Some(node_ref) => node_ref,
								None => return
							};
							let error_node: HtmlSpanElement = error_node_ref.unchecked_into();
							error_node.set_inner_text("");
						});

						let add_event_submission_handler = move |event: WebEvent| {
							event.prevent_default();

							let error_node_ref: DomNode = form_error_node.get();
							let error_node: HtmlSpanElement = error_node_ref.unchecked_into();

							let entered_event_name = (*new_event_name_signal.get()).clone();
							let events_index = events_names_index_signal.get();
							let Some(event_data) = events_index.get(&entered_event_name) else {
								error_node.set_inner_text("The entered event does not exist");
								return;
							};
							if group_events_signal.get().iter().any(|ev| ev.event.id == event_data.id) {
								error_node.set_inner_text("That event already has data for this group");
								return;
							}

							let new_event_permission = OptionalEventPermission { event: event_data.clone(), level: None };
							group_events_signal.modify().push(new_event_permission);
							new_event_name_signal.set(String::new());
						};

						let group_events_list_id = group_id.clone();
						view! {
							ctx,
							div {
								div(class="admin_group_header", on:click=group_header_click_handler) {
									input(bind:value=group_name_signal, ref=group_name_field)
								}
								div(class=*events_class.get()) {
									Keyed(
										iterable=group_events_signal,
										key=|event_permission| event_permission.event.id.clone(),
										view={
											let group_id = group_id.clone();
											move |ctx, event_permission| {
												let view_id = format!("admin_group_event_line_view-{}-{}", group_id, event_permission.event.id);
												let view_id_for = view_id.clone();
												let edit_id = format!("admin_group_event_line_edit-{}-{}", group_id, event_permission.event.id);
												let edit_id_for = edit_id.clone();

												let event_permission_signal = create_signal(ctx, event_permission.level);
												let event_view_signal = create_signal(ctx, event_permission.level.is_some());
												let event_edit_signal = create_signal(ctx, event_permission.level == Some(PermissionLevel::Edit));

												create_effect(ctx, || {
													let view_state = *event_view_signal.get();
													if !view_state {
														event_edit_signal.set(false);
													}
												});
												create_effect(ctx, || {
													let edit_state = *event_edit_signal.get();
													if edit_state {
														event_view_signal.set(true);
													}
												});
												create_effect(ctx, || {
													let view_state = *event_view_signal.get();
													let edit_state = *event_edit_signal.get();
													let new_level = match (view_state, edit_state) {
														(_, true) => Some(PermissionLevel::Edit),
														(true, false) => Some(PermissionLevel::View),
														_ => None
													};
													event_permission_signal.set(new_level);
												});
												create_effect(ctx, {
													let group_id = group_id.clone();
													move || {
														let new_level = *event_permission_signal.get();
														let mut modify_events = group_events_signal.modify();
														let event_data = modify_events.iter_mut().find(|event| event_permission.event.id == event.event.id).expect("Event being rendered exists in the group event data");
														event_data.level = new_level;
														updated_groups_signal.modify().insert(group_id.clone());
													}
												});
												create_effect(ctx, || {
													let new_level = *event_permission_signal.get();
													let (view_state, edit_state) = match new_level {
														Some(PermissionLevel::Edit) => (true, true),
														Some(PermissionLevel::View) => (true, false),
														None => (false, false)
													};
													event_edit_signal.set(edit_state);
													event_view_signal.set(view_state);
												});

												view! {
													ctx,
													div(class="admin_group_event_line") {
														div { (event_permission.event.name) }
														div {
															input(type="checkbox", id=view_id, bind:checked=event_view_signal)
															label(for=view_id_for) { "View" }
														}
														div {
															input(type="checkbox", id=edit_id, bind:checked=event_edit_signal)
															label(for=edit_id_for) { "Edit" }
														}
													}
												}
											}
										}
									)
									(if available_events_signal.get().is_empty() {
										view! { ctx, }
									} else {
										let events_list_id = format!("admin_group_event_list-{}", group_events_list_id);
										let events_list_id_ref = events_list_id.clone();
										view! {
											ctx,
											form(on:submit=add_event_submission_handler) {
												datalist(id=events_list_id) {
													Keyed(
														iterable=available_events_signal,
														key=|event| event.id.clone(),
														view=|ctx, event| view! { ctx, option(value=event.name) }
													)
												}
												input(placeholder="Add event", list=events_list_id_ref, bind:value=new_event_name_signal)
												button { "Add" }
												span(class="form-error", ref=form_error_node)
											}
										}
									})
								}
							}
						}
					}
				)
			}

			button(ref=submit_button) { "Update" }
			button(type="button", on:click=cancel_button_handler, ref=cancel_button) { "Cancel" }
			button(type="button", on:click=add_new_button_handler) { "Add New Group" }
		}
	}
}

#[component]
pub fn AdminManageGroupsView<G: Html>(ctx: Scope<'_>) -> View<G> {
	view! {
		ctx,
		Suspense(
			fallback=view! { ctx, "Loading permission groups..." }
		) {
			AdminManageGroupsLoadedView
		}
	}
}
