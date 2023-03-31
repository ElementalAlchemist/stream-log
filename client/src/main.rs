use futures::lock::Mutex;
use futures::StreamExt;
use gloo_net::websocket::futures::WebSocket;
use stream_log_shared::messages::initial::{InitialMessage, UserDataLoad};
use stream_log_shared::SYNC_VERSION;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::{HistoryIntegration, Route, Router};
use websocket::websocket_endpoint;

mod color_utils;
mod components;
mod event_type_colors;
mod pages;
mod subscriptions;
mod websocket;
use components::error_display::ErrorDisplay;
use components::user_info_bar::UserInfoBar;
use pages::admin::assign_event_types::AdminManageEventTypesForEventsView;
use pages::admin::assign_groups::AssignUsersToGroupsView;
use pages::admin::manage_editors::AdminManageEditorsView;
use pages::admin::manage_event_types::AdminManageEventTypesView;
use pages::admin::manage_events::AdminManageEventsView;
use pages::admin::manage_groups::AdminManageGroupsView;
use pages::admin::manage_tags::AdminManageTagsView;
use pages::admin::manage_users::AdminManageUsersView;
use pages::event_log::EventLogView;
use pages::event_selection::EventSelectionView;
use pages::not_found::NotFoundView;
use pages::register::RegistrationView;
use pages::register_complete::RegistrationCompleteView;
use pages::user_profile::UserProfileView;
use subscriptions::{process_messages, DataSignals};
use websocket::read_websocket;

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
	#[to("/admin/events")]
	AdminEventManager,
	#[to("/admin/users")]
	AdminUserManager,
	#[to("/admin/groups")]
	AdminPermissionGroupManager,
	#[to("/admin/assign_groups")]
	AdminUserGroupAssignmentManager,
	#[to("/admin/event_types")]
	AdminEventTypeManager,
	#[to("/admin/assign_event_types")]
	AdminEventTypesForEventManager,
	#[to("/admin/tags")]
	AdminTagsManager,
	#[to("/admin/editors")]
	AdminEditorsManager,
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

	let user_data = match initial_message.user_data {
		UserDataLoad::User(user_data) => Some(user_data),
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
	provide_context_ref(ctx, create_signal(ctx, user_data));

	// Assuming the WASM client for this might multithread at any point in the future is probably way overkill.
	// That said, we need to await for any websocket operations anyway, so a locking wrapper doesn't hurt us.
	// Since contention is unlikely, this shouldn't introduce any significant delay.
	let ws = Mutex::new(ws_write);
	provide_context(ctx, ws);

	let client_data = DataSignals::new(ctx);
	provide_context(ctx, create_rc_signal(client_data));

	spawn_local_scoped(ctx, process_messages(ctx, ws_read));

	view! {
		ctx,
		ErrorDisplay
		Router(
			integration=HistoryIntegration::new(),
			view=|ctx, route: &ReadSignal<AppRoutes>| {
				log::info!("Navigating to route: {:?}", route.get());
				view! {
					ctx,
					UserInfoBar {} // This must remain in the router so its links can be handled by the router
					(match route.get().as_ref() {
						AppRoutes::EventSelection => view! { ctx, EventSelectionView },
						AppRoutes::Register => view! { ctx, RegistrationView },
						AppRoutes::RegistrationComplete => view! { ctx, RegistrationCompleteView },
						AppRoutes::EventLog(id) => view! { ctx, EventLogView(id=id.clone()) },
						AppRoutes::AdminEventManager => view! { ctx, AdminManageEventsView },
						AppRoutes::AdminUserManager => view! { ctx, AdminManageUsersView },
						AppRoutes::AdminPermissionGroupManager => view! { ctx, AdminManageGroupsView },
						AppRoutes::AdminUserGroupAssignmentManager => view! { ctx, AssignUsersToGroupsView },
						AppRoutes::AdminEventTypeManager => view! { ctx, AdminManageEventTypesView },
						AppRoutes::AdminEventTypesForEventManager => view! { ctx, AdminManageEventTypesForEventsView },
						AppRoutes::AdminTagsManager => view! { ctx, AdminManageTagsView },
						AppRoutes::AdminEditorsManager => view! { ctx, AdminManageEditorsView },
						AppRoutes::UserProfile => view! { ctx, UserProfileView },
						AppRoutes::NotFound => view! { ctx, NotFoundView }
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
