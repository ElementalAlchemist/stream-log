// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use async_std::fs;
use async_std::sync::{Arc, Mutex};
use clap::Parser;
use miette::IntoDiagnostic;
use tide::http::cookies::SameSite;
use tide::sessions::SessionMiddleware;
use tide::{Body, Server};
use tide_openidconnect::{
	ClientId, ClientSecret, IssuerUrl, OpenIdConnectMiddleware, OpenIdConnectRouteExt, RedirectUrl,
};
use tide_websockets::WebSocket;

mod api;

mod args;
use args::CliArgs;

mod config;
use config::parse_config;

mod data_sync;
use data_sync::connection::handle_connection;
use data_sync::new_event_entries::NewEventEntries;
use data_sync::SubscriptionManager;

mod database;
use database::{connect_db, run_embedded_migrations};

mod session;
use session::DatabaseSessionStore;

mod websocket_msg;

mod models;
mod schema;

fn establish_alternate_route(app: &mut Server<()>, path: &str) -> miette::Result<()> {
	app.at(path)
		.authenticated()
		.serve_file("static/index.html")
		.into_diagnostic()
}

#[async_std::main]
async fn main() -> miette::Result<()> {
	let args = CliArgs::parse();

	let config = Arc::new(parse_config(&args.config).await?);

	let db_connection_pool = connect_db(&config)?;
	run_embedded_migrations(&db_connection_pool)?;

	if args.migrations_only {
		return Ok(());
	}

	tide::log::start();

	let subscription_manager = Arc::new(Mutex::new(SubscriptionManager::new()));
	let new_entries = Arc::new(Mutex::new(NewEventEntries::default()));

	let mut app = tide::new();

	let session_middleware = {
		let session_secret = fs::read(&config.session_secret_key_file).await.into_diagnostic()?;
		SessionMiddleware::new(DatabaseSessionStore::new(db_connection_pool.clone()), &session_secret)
			.with_same_site_policy(SameSite::Lax)
	};
	app.with(session_middleware);

	let openid_config = tide_openidconnect::Config {
		issuer_url: IssuerUrl::new(config.openid.endpoint.clone()).into_diagnostic()?,
		client_id: ClientId::new(config.openid.client_id.clone()),
		client_secret: ClientSecret::new(config.openid.secret.clone()),
		redirect_url: RedirectUrl::new(config.openid.response_url.clone()).into_diagnostic()?,
		idp_logout_url: Some(config.openid.logout_url.clone()),
	};
	app.with(OpenIdConnectMiddleware::new(&openid_config).await);

	api::add_routes(&mut app, db_connection_pool.clone(), Arc::clone(&subscription_manager))?;

	app.at("/ws").authenticated().get(WebSocket::new({
		let subscription_manager = Arc::clone(&subscription_manager);
		let new_entries = Arc::clone(&new_entries);
		move |request, stream| {
			let db_connection_pool = db_connection_pool.clone();
			let subscription_manager = Arc::clone(&subscription_manager);
			let new_entries = Arc::clone(&new_entries);
			async move {
				handle_connection(
					db_connection_pool.clone(),
					request,
					stream,
					subscription_manager,
					new_entries,
				)
				.await
			}
		}
	}));

	if let Some(favicon_file_path) = config.favicon_file.as_ref() {
		app.at("/favicon.ico").serve_file(favicon_file_path).into_diagnostic()?;
	}

	app.at("/")
		.authenticated()
		.get(|_| async { Ok(Body::from_file("static/index.html").await?) })
		.serve_dir("static/")
		.into_diagnostic()?;

	establish_alternate_route(&mut app, "/register")?;
	establish_alternate_route(&mut app, "/register_complete")?;
	establish_alternate_route(&mut app, "/log/:id")?;
	establish_alternate_route(&mut app, "/log/:id/tags")?;
	establish_alternate_route(&mut app, "/log/:id/entry_types")?;
	establish_alternate_route(&mut app, "/log/:event_id/page/:page_id")?;
	establish_alternate_route(&mut app, "/admin/events")?;
	establish_alternate_route(&mut app, "/admin/users")?;
	establish_alternate_route(&mut app, "/admin/groups")?;
	establish_alternate_route(&mut app, "/admin/assign_groups")?;
	establish_alternate_route(&mut app, "/admin/event_types")?;
	establish_alternate_route(&mut app, "/admin/assign_event_types")?;
	establish_alternate_route(&mut app, "/admin/editors")?;
	establish_alternate_route(&mut app, "/admin/tags")?;
	establish_alternate_route(&mut app, "/admin/applications")?;
	establish_alternate_route(&mut app, "/admin/info_pages")?;
	establish_alternate_route(&mut app, "/user_profile")?;

	app.listen(&config.listen.addr).await.into_diagnostic()?;

	tide::log::info!("Initiating shutdown");

	let mut shutdown_subscription_manager = SubscriptionManager::new();
	{
		let mut subscription_manager = subscription_manager.lock().await;
		std::mem::swap(&mut *subscription_manager, &mut shutdown_subscription_manager);
	}
	shutdown_subscription_manager.shutdown().await;

	Ok(())
}
