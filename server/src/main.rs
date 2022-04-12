use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::sync::Arc;
use async_std::task;
use miette::IntoDiagnostic;
use tide::prelude::*;
use tide::Body;
use tide_websockets::WebSocket;

mod config;
use config::parse_config;

#[async_std::main]
async fn main() -> miette::Result<()> {
	let config = Arc::new(parse_config()?);

	tide::log::start();

	let mut app = tide::new();
	app.at("/ws")
		.with(WebSocket::new(|request, mut stream| async move {
			stream.send_string(String::from("Unauthorized")).await?;
			Ok(())
		}))
		.get(|_| async move { Ok("Must be a websocket request") });
	app.at("/")
		.get(|_| async { Ok(Body::from_file("static/index.html").await?) })
		.serve_dir("static/")
		.into_diagnostic()?;
	app.listen(&config.listen.addr).await.into_diagnostic()?;

	Ok(())
}
