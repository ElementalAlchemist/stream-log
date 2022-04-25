use async_std::fs;
use async_std::sync::{Arc, Mutex};
use miette::IntoDiagnostic;
use tide::http::cookies::SameSite;
use tide::sessions::{MemoryStore, SessionMiddleware};
use tide::Body;
use tide_openidconnect::{
	ClientId, ClientSecret, IssuerUrl, OpenIdConnectMiddleware, OpenIdConnectRouteExt, RedirectUrl,
};
use tide_websockets::WebSocket;

mod config;
use config::parse_config;

mod data_sync;
use data_sync::connection::handle_connection;

mod database;
use database::connect_db;

mod websocket_msg;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
mod diesel_types;
mod models;
mod schema;

embed_migrations!();

#[async_std::main]
async fn main() -> miette::Result<()> {
	let config = Arc::new(parse_config()?);

	tide::log::start();

	let db_connection = connect_db(&config)?;
	embedded_migrations::run(&db_connection).into_diagnostic()?;
	let db_connection = Arc::new(Mutex::new(db_connection));

	let mut app = tide::new();

	let session_middleware = {
		let session_secret = fs::read(&config.session_secret_key_file).await.into_diagnostic()?;
		SessionMiddleware::new(MemoryStore::new(), &session_secret).with_same_site_policy(SameSite::Lax)
	};
	app.with(session_middleware);

	let openid_config = tide_openidconnect::Config {
		issuer_url: IssuerUrl::new(String::from("https://accounts.google.com")).into_diagnostic()?,
		client_id: ClientId::new(config.google_credentials.client_id.clone()),
		client_secret: ClientSecret::new(config.google_credentials.secret.clone()),
		redirect_url: RedirectUrl::new(config.openid_response_url.clone()).into_diagnostic()?,
		idp_logout_url: None,
	};
	app.with(OpenIdConnectMiddleware::new(&openid_config).await);

	app.at("/ws").authenticated().get(WebSocket::new({
		let config = Arc::clone(&config);
		let db_connection = Arc::clone(&db_connection);
		move |request, stream| {
			let config = Arc::clone(&config);
			let db_connection = Arc::clone(&db_connection);
			async move { handle_connection(config, db_connection, request, stream).await }
		}
	}));

	app.at("/")
		.authenticated()
		.get(|_| async { Ok(Body::from_file("static/index.html").await?) })
		.serve_dir("static/")
		.into_diagnostic()?;

	app.listen(&config.listen.addr).await.into_diagnostic()?;

	Ok(())
}
