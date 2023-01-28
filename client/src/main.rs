use futures::lock::Mutex;
use gloo_net::websocket::futures::WebSocket;
use stream_log_shared::messages::initial::{InitialMessage, UserDataLoad};
use stream_log_shared::SYNC_VERSION;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::{HistoryIntegration, Route, Router};
use websocket::websocket_endpoint;

mod color_utils;
mod components;
mod event_type_colors;
mod pages;
mod websocket;
use components::user_info_bar::UserInfoBar;
use pages::admin::assign_event_types::AdminManageEventTypesForEventsView;
use pages::admin::assign_groups::AssignUsersToGroupsView;
use pages::admin::manage_event_types::AdminManageEventTypesView;
use pages::admin::manage_events::AdminManageEventsView;
use pages::admin::manage_groups::AdminManageGroupsView;
use pages::admin::manage_tags::AdminManageTagsView;
use pages::admin::manage_users::AdminManageUsersView;
use pages::error::{ErrorData, ErrorView};
use pages::event_selection::EventSelectionView;
use pages::not_found::NotFoundView;
use pages::register::RegistrationView;
use pages::register_complete::RegistrationCompleteView;
use pages::user_profile::UserProfileView;
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
	#[to("/user_profile")]
	UserProfile,
	#[to("/error")]
	Error,
	#[not_found]
	NotFound,
}

#[component]
async fn App<G: Html>(ctx: Scope<'_>) -> View<G> {
	let error_data: Option<ErrorData> = None;
	provide_context_ref(ctx, create_signal(ctx, error_data));

	let ws = WebSocket::open(websocket_endpoint().as_str());
	let mut ws = match ws {
		Ok(ws) => ws,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Unable to load/operate: Failed to form a websocket connection",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	let initial_message: InitialMessage = match read_websocket(&mut ws).await {
		Ok(msg) => msg,
		Err(error) => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new_with_error(
				"Unable to load/operate: Failed to read initial info message",
				error,
			)));
			return view! { ctx, ErrorView };
		}
	};

	if initial_message.sync_version != SYNC_VERSION {
		let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
		error_signal.set(Some(ErrorData::new("A mismatch in communication protocols occurred between the lient and the server. Please refresh the page. If the problem persists, please contact an administrator.")));
		return view! { ctx, ErrorView };
	}

	let user_data = match initial_message.user_data {
		UserDataLoad::User(user_data) => Some(user_data),
		UserDataLoad::NewUser => None,
		UserDataLoad::MissingId => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new(
				"An error occurred reading user data. Please log in again.",
			)));
			return view! { ctx, ErrorView };
		}
		UserDataLoad::Error => {
			let error_signal: &Signal<Option<ErrorData>> = use_context(ctx);
			error_signal.set(Some(ErrorData::new(
				"An error occurred with logging in. Please contact an administrator regarding this issue.",
			)));
			return view! { ctx, ErrorView };
		}
	};
	provide_context_ref(ctx, create_signal(ctx, user_data));

	// Assuming the WASM client for this might multithread at any point in the future is probably way overkill.
	// That said, we need to await for any websocket operations anyway, so a locking wrapper doesn't hurt us.
	// Since contention is unlikely, this shouldn't introduce any significant delay.
	let ws = Mutex::new(ws);
	provide_context(ctx, ws);

	view! {
		ctx,
		Router(
			integration=HistoryIntegration::new(),
			view=|ctx, route: &ReadSignal<AppRoutes>| {
				log::info!("Navigating to route: {:?}", route.get());
				view! {
					ctx,
					UserInfoBar {}
					(match route.get().as_ref() {
						AppRoutes::EventSelection => view! { ctx, EventSelectionView },
						AppRoutes::Register => view! { ctx, RegistrationView },
						AppRoutes::RegistrationComplete => view! { ctx, RegistrationCompleteView },
						AppRoutes::EventLog(id) => todo!(),
						AppRoutes::AdminEventManager => view! { ctx, AdminManageEventsView },
						AppRoutes::AdminUserManager => view! { ctx, AdminManageUsersView },
						AppRoutes::AdminPermissionGroupManager => view! { ctx, AdminManageGroupsView },
						AppRoutes::AdminUserGroupAssignmentManager => view! { ctx, AssignUsersToGroupsView },
						AppRoutes::AdminEventTypeManager => view! { ctx, AdminManageEventTypesView },
						AppRoutes::AdminEventTypesForEventManager => view! { ctx, AdminManageEventTypesForEventsView },
						AppRoutes::AdminTagsManager => view! { ctx, AdminManageTagsView },
						AppRoutes::UserProfile => view! { ctx, UserProfileView },
						AppRoutes::Error => view! { ctx, ErrorView },
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
