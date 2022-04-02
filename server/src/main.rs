use async_std::fs;
use async_std::io::ErrorKind;
use async_std::net::{TcpListener, TcpStream};
use async_std::path::Path;
use async_std::prelude::*;
use async_std::task;
use http_types::{Method, Response, StatusCode};
use miette::{IntoDiagnostic, Result};

mod config;
use config::parse_config;

mod file_types;

mod web;
use web::handle_request;

#[async_std::main]
async fn main() -> Result<()> {
	let listener = TcpListener::bind(("127.0.0.1", 8080)).await.into_diagnostic()?;

	let mut incoming = listener.incoming();
	while let Some(stream) = incoming.next().await {
		let stream = stream.into_diagnostic()?;
		task::spawn(async {
			if let Err(err) = handle_request(stream).await {
				eprintln!("{}", err);
			}
		});
	}
	Ok(())
}
