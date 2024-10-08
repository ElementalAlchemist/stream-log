// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use futures::lock::Mutex;
use futures::task::Waker;
use futures::StreamExt;
use gloo_net::websocket::futures::WebSocket;
use std::collections::HashMap;
use stream_log_shared::messages::initial::{InitialMessage, UserDataLoad};
use stream_log_shared::SYNC_VERSION;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::{HistoryIntegration, Route, Router};

mod color_utils;
mod components;
mod entry_type_colors;
mod entry_utils;
mod page_utils;
mod pages;
mod subscriptions;
mod websocket;
use components::error_display::ErrorDisplay;
use components::user_info_bar::{EventId, UserInfoBar};
use page_utils::set_page_title;
use pages::admin::assign_entry_types::AdminManageEntryTypesForEventsView;
use pages::admin::assign_groups::AssignUsersToGroupsView;
use pages::admin::manage_applications::AdminApplicationsView;
use pages::admin::manage_editors::AdminManageEditorsView;
use pages::admin::manage_entry_types::AdminManageEntryTypesView;
use pages::admin::manage_events::AdminManageEventsView;
use pages::admin::manage_groups::AdminManageGroupsView;
use pages::admin::manage_info_pages::AdminInfoPagesView;
use pages::admin::manage_tabs::AdminManageEventLogTabsView;
use pages::admin::manage_users::AdminManageUsersView;
use pages::event_log::entry_types::EventLogEntryTypesView;
use pages::event_log::info_page::EventLogInfoPageView;
use pages::event_log::log::EventLogView;
use pages::event_log::tags::EventLogTagsView;
use pages::event_selection::EventSelectionView;
use pages::not_found::NotFoundView;
use pages::register::RegistrationView;
use pages::register_complete::RegistrationCompleteView;
use pages::user_profile::UserProfileView;
use subscriptions::manager::SubscriptionManager;
use subscriptions::{initial_events_sort, process_messages, DataSignals};
use websocket::{read_websocket, websocket_endpoint, WebSocketSendStream};

#[derive(Debug, Route)]
enum AppRoutes {
	#[to("/")]
	EventSelection,
	#[to("/register")]
	Register,
	#[to("/register_complete")]
	RegistrationComplete,
	#[to("/log/<id>")]
	EventLog(String),
	#[to("/log/<id>/tags")]
	EventLogTags(String),
	#[to("/log/<id>/entry_types")]
	EventLogEntryTypes(String),
	#[to("/log/<event_id>/page/<page_id>")]
	EventLogInfoPage(String, String),
	#[to("/admin/events")]
	AdminEventManager,
	#[to("/admin/users")]
	AdminUserManager,
	#[to("/admin/groups")]
	AdminPermissionGroupManager,
	#[to("/admin/assign_groups")]
	AdminUserGroupAssignmentManager,
	#[to("/admin/event_types")]
	AdminEntryTypeManager,
	#[to("/admin/assign_event_types")]
	AdminEntryTypesForEventManager,
	#[to("/admin/editors")]
	AdminEditorsManager,
	#[to("/admin/tabs")]
	AdminEventLogTabsManager,
	#[to("/admin/applications")]
	AdminApplicationsManager,
	#[to("/admin/info_pages")]
	AdminInfoPagesManager,
	#[to("/user_profile")]
	UserProfile,
	#[not_found]
	NotFound,
}

#[component]
async fn App<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws = WebSocket::open(websocket_endpoint().as_str());
	let ws = match ws {
		Ok(ws) => ws,
		Err(error) => {
			return view! {
				ctx,
				div(id="fatal_startup_error") {
					div(id="fatal_startup_error_description") {
						"Unable to load/operate: Failed to form a websocket connection"
					}
					div(id="fatal_startup_error_details") { (error) }
				}
			}
		}
	};

	let (ws_write, mut ws_read) = ws.split();

	let initial_message: InitialMessage = match read_websocket(&mut ws_read).await {
		Ok(msg) => msg,
		Err(error) => {
			return view! {
				ctx,
				div(id="fatal_startup_error") {
					div(id="fatal_startup_error_description") {
						"Unable to load/operate: Failed to read initial info message"
					}
					div(id="fatal_startup_error_details") { (error) }
				}
			}
		}
	};

	if initial_message.sync_version != SYNC_VERSION {
		return view! {
			ctx,
			div(id="fatal_startup_error") {
				div(id="fatal_startup_error_description") {
					"A mismatch in communication protocols occurred between the client and the server. Please refresh the page. If the problem persists, please contact an administrator."
				}
			}
		};
	}

	let initial_data = match initial_message.user_data {
		UserDataLoad::User(user_data, available_events) => Some((user_data, available_events)),
		UserDataLoad::NewUser => None,
		UserDataLoad::MissingId => {
			return view! {
				ctx,
				div(id="fatal_startup_error") {
					div(id="fatal_startup_error_description") {
						"An error occurred reading user data. Please log in again."
					}
				}
			};
		}
		UserDataLoad::Error => {
			return view! {
				ctx,
				div(id="fatal_startup_error") {
					div(id="fatal_startup_error_description") {
						"An error occurred logging in. Please contact an administrator."
					}
				}
			}
		}
	};
	let (user_data, available_events) = if let Some((user, mut events)) = initial_data {
		initial_events_sort(&mut events);
		(Some(user), Some(events))
	} else {
		(None, None)
	};
	provide_context_ref(ctx, create_signal(ctx, user_data));

	// Assuming the WASM client for this might multithread at any point in the future is probably way overkill.
	// That said, we need to await for any websocket operations anyway, so a locking wrapper doesn't hurt us.
	// Since contention is unlikely, this shouldn't introduce any significant delay.
	let ws = WebSocketSendStream::new(ws_write);
	let ws = Mutex::new(ws);
	provide_context(ctx, ws);

	let mut client_data = DataSignals::new();
	if let Some(events) = available_events {
		client_data.available_events = create_rc_signal(events);
	}
	provide_context(ctx, client_data);
	let subscription_manager = Mutex::new(SubscriptionManager::default());
	provide_context(ctx, subscription_manager);
	let event_wakers: HashMap<String, Vec<Waker>> = HashMap::new();
	provide_context_ref(ctx, create_signal(ctx, event_wakers));

	spawn_local_scoped(ctx, process_messages(ctx, ws_read));

	let current_event_id: &Signal<Option<EventId>> = create_signal(ctx, None);
	provide_context_ref(ctx, current_event_id);

	view! {
		ctx,
		ErrorDisplay
		Router(
			integration=HistoryIntegration::new(),
			view=move |ctx, route: &ReadSignal<AppRoutes>| {
				view! {
					ctx,
					UserInfoBar {} // This must remain in the router so its links can be handled by the router
					({
						log::info!("Navigating to route: {:?}", route.get());

						// Default the window title in case the page doesn't support/set it
						set_page_title("Stream Log");

						match route.get().as_ref() {
							AppRoutes::EventLog(id) | AppRoutes::EventLogTags(id) | AppRoutes::EventLogEntryTypes(id) | AppRoutes::EventLogInfoPage(id, _) => current_event_id.set(Some(EventId::new(id.clone()))),
							_ => current_event_id.set(None)
						}
						match route.get().as_ref() {
							AppRoutes::EventSelection => view! { ctx, EventSelectionView },
							AppRoutes::Register => view! { ctx, RegistrationView },
							AppRoutes::RegistrationComplete => view! { ctx, RegistrationCompleteView },
							AppRoutes::EventLog(id) => view! { ctx, EventLogView(id=id.clone()) },
							AppRoutes::EventLogTags(id) => view! { ctx, EventLogTagsView(id=id.clone()) },
							AppRoutes::EventLogEntryTypes(id) => view! { ctx, EventLogEntryTypesView(id=id.clone()) },
							AppRoutes::EventLogInfoPage(event_id, page_id) => view! { ctx, EventLogInfoPageView(event_id=event_id.clone(),page_id=page_id.clone()) },
							AppRoutes::AdminEventManager => view! { ctx, AdminManageEventsView },
							AppRoutes::AdminUserManager => view! { ctx, AdminManageUsersView },
							AppRoutes::AdminPermissionGroupManager => view! { ctx, AdminManageGroupsView },
							AppRoutes::AdminUserGroupAssignmentManager => view! { ctx, AssignUsersToGroupsView },
							AppRoutes::AdminEntryTypeManager => view! { ctx, AdminManageEntryTypesView },
							AppRoutes::AdminEntryTypesForEventManager => view! { ctx, AdminManageEntryTypesForEventsView },
							AppRoutes::AdminEditorsManager => view! { ctx, AdminManageEditorsView },
							AppRoutes::AdminEventLogTabsManager => view! { ctx, AdminManageEventLogTabsView },
							AppRoutes::AdminApplicationsManager => view! { ctx, AdminApplicationsView },
							AppRoutes::AdminInfoPagesManager => view! { ctx, AdminInfoPagesView },
							AppRoutes::UserProfile => view! { ctx, UserProfileView },
							AppRoutes::NotFound => view! { ctx, NotFoundView }
						}
					})
				}
			}
		)
	}
}

fn main() {
	console_error_panic_hook::set_once();
	wasm_logger::init(wasm_logger::Config::default());

	sycamore::render(|ctx| {
		view! {
			ctx,
			Suspense(fallback=view! { ctx, "Causing the enloadening..." }) {
				App
			}
		}
	});
}
