use crate::subscriptions::errors::ErrorData;
use crate::subscriptions::manager::SubscriptionManager;
use crate::subscriptions::DataSignals;
use crate::websocket::WebSocketSendStream;
use futures::lock::Mutex;
use gloo_net::websocket::Message;
use stream_log_shared::messages::admin::{AdminApplicationUpdate, Application};
use stream_log_shared::messages::subscriptions::{SubscriptionTargetUpdate, SubscriptionType};
use stream_log_shared::messages::user::UserData;
use stream_log_shared::messages::FromClientMessage;
use sycamore::futures::spawn_local_scoped;
use sycamore::prelude::*;
use sycamore::suspense::Suspense;
use sycamore_router::navigate;
use web_sys::Event as WebEvent;

#[component]
async fn AdminApplicationsLoadedView<G: Html>(ctx: Scope<'_>) -> View<G> {
	let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
	let mut ws = ws_context.lock().await;

	let data: &DataSignals = use_context(ctx);
	data.show_application_auth_keys.set(Vec::new());

	let set_subscription_result = {
		let subscription_manager: &Mutex<SubscriptionManager> = use_context(ctx);
		let mut subscription_manager = subscription_manager.lock().await;
		subscription_manager
			.set_subscription(SubscriptionType::AdminApplications, &mut ws)
			.await
	};
	if let Err(error) = set_subscription_result {
		data.errors.modify().push(ErrorData::new_with_error(
			"Failed to subscribe to application data.",
			error,
		));
	}

	let read_applications = create_memo(ctx, || (*data.all_applications.get()).clone());
	let read_auth_keys = create_memo(ctx, || (*data.show_application_auth_keys.get()).clone());

	let new_application_name = create_signal(ctx, String::new());
	let new_application_read_log = create_signal(ctx, false);
	let new_application_write_links = create_signal(ctx, false);
	let submit_new_application = move |event: WebEvent| {
		event.prevent_default();

		let name = (*new_application_name.get()).clone();
		if name.is_empty() {
			return;
		}

		let read_log = *new_application_read_log.get();
		let write_links = *new_application_write_links.get();

		let new_application = Application {
			id: String::new(),
			name,
			read_log,
			write_links,
		};

		spawn_local_scoped(ctx, async move {
			let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
			let mut ws = ws_context.lock().await;

			let message =
				FromClientMessage::SubscriptionMessage(Box::new(SubscriptionTargetUpdate::AdminApplicationsUpdate(
					AdminApplicationUpdate::UpdateApplication(new_application),
				)));
			let message_json = match serde_json::to_string(&message) {
				Ok(msg) => msg,
				Err(error) => {
					let data: &DataSignals = use_context(ctx);
					data.errors.modify().push(ErrorData::new_with_error(
						"Failed to serialize new application message.",
						error,
					));
					return;
				}
			};

			let send_result = ws.send(Message::Text(message_json)).await;
			if let Err(error) = send_result {
				let data: &DataSignals = use_context(ctx);
				data.errors.modify().push(ErrorData::new_with_error(
					"Failed to send new application message.",
					error,
				));
			}

			new_application_name.set(String::new());
			new_application_read_log.set(false);
			new_application_write_links.set(false);
		});
	};

	view! {
		ctx,
		div(id="admin_manage_applications") {
			Keyed(
				iterable=read_applications,
				key=|app| app.id.clone(),
				view=|ctx, application| {
					let entered_name = create_signal(ctx, application.name.clone());
					let entered_read_log = create_signal(ctx, application.read_log);
					let entered_write_links = create_signal(ctx, application.write_links);

					let update_application = {
						let application = application.clone();
						move |event: WebEvent| {
							event.prevent_default();

							let name = (*entered_name.get()).clone();
							if name.is_empty() {
								return;
							}
							let read_log = *entered_read_log.get();
							let write_links = *entered_write_links.get();

							let updated_application = Application { id: application.id.clone(), name, read_log, write_links };
							spawn_local_scoped(ctx, async move {
								let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								let message = FromClientMessage::SubscriptionMessage(
									Box::new(
										SubscriptionTargetUpdate::AdminApplicationsUpdate(
											AdminApplicationUpdate::UpdateApplication(
												updated_application
											)
										)
									)
								);
								let message_json = match serde_json::to_string(&message) {
									Ok(msg) => msg,
									Err(error) => {
										let data: &DataSignals = use_context(ctx);
										data.errors.modify().push(ErrorData::new_with_error("Failed to serialize application update message.", error));
										return;
									}
								};

								let send_result = ws.send(Message::Text(message_json)).await;
								if let Err(error) = send_result {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to send application update message.", error));
								}
							});
						}
					};

					let reset_auth_key = {
						let application = application.clone();
						move |_event: WebEvent| {
							let application = application.clone();
							spawn_local_scoped(ctx, async move {
								let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								let message = FromClientMessage::SubscriptionMessage(
									Box::new(
										SubscriptionTargetUpdate::AdminApplicationsUpdate(
											AdminApplicationUpdate::ResetAuthToken(
												application
											)
										)
									)
								);
								let message_json = match serde_json::to_string(&message) {
									Ok(msg) => msg,
									Err(error) => {
										let data: &DataSignals = use_context(ctx);
										data.errors.modify().push(ErrorData::new_with_error("Failed to serialize application auth key reset message.", error));
										return;
									}
								};

								let send_result = ws.send(Message::Text(message_json)).await;
								if let Err(error) = send_result {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to send application auth key reset message.", error));
								}
							});
						}
					};

					let revoke_application = {
						let application = application.clone();
						move |_event: WebEvent| {
							let application = application.clone();
							spawn_local_scoped(ctx, async move {
								let ws_context: &Mutex<WebSocketSendStream> = use_context(ctx);
								let mut ws = ws_context.lock().await;

								let message = FromClientMessage::SubscriptionMessage(
									Box::new(
										SubscriptionTargetUpdate::AdminApplicationsUpdate(
											AdminApplicationUpdate::RevokeApplication(
												application
											)
										)
									)
								);
								let message_json = match serde_json::to_string(&message) {
									Ok(msg) => msg,
									Err(error) => {
										let data: &DataSignals = use_context(ctx);
										data.errors.modify().push(ErrorData::new_with_error("Failed to serialize application revoke message.", error));
										return;
									}
								};

								let send_result = ws.send(Message::Text(message_json)).await;
								if let Err(error) = send_result {
									let data: &DataSignals = use_context(ctx);
									data.errors.modify().push(ErrorData::new_with_error("Failed to send application revoke message.", error));
								}
							});
						}
					};

					view! {
						ctx,
						form(class="admin_manage_applications_application", on:submit=update_application) {
							div(class="admin_manage_applications_application_name") {
								input(bind:value=entered_name, placeholder="Name")
							}
							div(class="admin_manage_applications_application_read_log") {
								label {
									"Read Log"
									input(type="checkbox", bind:checked=entered_read_log)
								}
							}
							div(class="admin_manage_applications_application_write_links") {
								label {
									"Write Links"
									input(type="checkbox", bind:checked=entered_write_links)
								}
							}
							div(class="admin_manage_applications_application_update") {
								button(type="submit") { "Update" }
							}
							div(class="admin_manage_applications_application_reset_key") {
								button(type="button", on:click=reset_auth_key) { "Reset Key" }
							}
							div(class="admin_manage_applications_application_revoke") {
								button(type="button", on:click=revoke_application) { "Revoke Application" }
							}
						}
					}
				}
			)
		}

		h1 { "Authorization Keys" }
		p { "When an application is created or its key is reset, the key is shown here one time. Authorization keys shown here will not be shown again once you leave this page." }
		table(id="admin_manage_applications_keys") {
			tr {
				th { "Application" }
				th { "Authorization Key" }
			}
			Keyed(
				iterable=read_auth_keys,
				key=|(_, auth_key)| auth_key.clone(),
				view=|ctx, (application, auth_key)| {
					view! {
						ctx,
						tr {
							td { (application.name) }
							td { (auth_key) }
						}
					}
				}
			)
		}

		h1 { "New Application" }
		form(id="admin_manage_applications_new", on:submit=submit_new_application) {
			div {
				input(bind:value=new_application_name, placeholder="Name")
			}
			div {
				label {
					"Read Log"
					input(type="checkbox", bind:checked=new_application_read_log)
				}
			}
			div {
				label {
					"Write Links"
					input(type="checkbox", bind:checked=new_application_write_links)
				}
			}
			button(type="submit") { "Add Application" }
		}
	}
}

#[component]
pub fn AdminApplicationsView<G: Html>(ctx: Scope<'_>) -> View<G> {
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
		Suspense(fallback=view! { ctx, "Loading applications..." }) {
			AdminApplicationsLoadedView
		}
	}
}
