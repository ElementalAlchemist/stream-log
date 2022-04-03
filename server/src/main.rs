use async_std::net::TcpListener;
use async_std::prelude::*;
use async_std::task;
use miette::IntoDiagnostic;

mod config;
mod file_types;
mod web;
use config::parse_config;
use web::handle_request;

#[async_std::main]
async fn main() -> miette::Result<()> {
	let config = parse_config()?;
	let listener = TcpListener::bind(&config.listen.addr).await.into_diagnostic()?;
	let mut incoming_listener = listener.incoming();
	while let Some(stream) = incoming_listener.next().await {
		let stream = stream.into_diagnostic()?;
		task::spawn(async {
			if let Err(err) = handle_request(stream).await {
				eprintln!("{}", err);
			}
		});
	}
	Ok(())
}
