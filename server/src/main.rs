use async_std::fs;
use async_std::sync::Arc;
use miette::IntoDiagnostic;
use stream_log_shared::messages::initial::{InitialMessage, InitialMessageUnauthorized};
use tide::http::cookies::SameSite;
use tide::prelude::*;
use tide::sessions::{MemoryStore, SessionMiddleware};
use tide::{Body, Request};
use tide_openidconnect::{ClientId, ClientSecret, IssuerUrl, OpenIdConnectMiddleware, OpenIdConnectRequestExt, OpenIdConnectRouteExt, RedirectUrl};
use tide_websockets::WebSocket;

mod config;
use config::parse_config;

#[async_std::main]
async fn main() -> miette::Result<()> {
	let config = Arc::new(parse_config()?);

	tide::log::start();

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
		redirect_url: RedirectUrl::new(String::from(config.openid_response_url.clone())).into_diagnostic()?,
		idp_logout_url: None,
	};
	app.with(OpenIdConnectMiddleware::new(&openid_config).await);

	let ws_config = Arc::clone(&config);
	app.at("/ws")
		.authenticated()
		.with(WebSocket::new(move |request, mut stream| {
			let config = Arc::clone(&ws_config);
			async move {
				let message = InitialMessage::Welcome;
				stream.send_json(&message).await?;
				Ok(())
			}
		}))
		.get(|_| async move { Ok("Must be a websocket request") });

	app.at("/")
		.authenticated()
		.get(|_| async { Ok(Body::from_file("static/index.html").await?) })
		.serve_dir("static/")
		.into_diagnostic()?;

	app.listen(&config.listen.addr).await.into_diagnostic()?;

	Ok(())
}
