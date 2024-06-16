use async_std::fs;
use knuffel::Decode;
use miette::{IntoDiagnostic, Result};

pub async fn parse_config(config_path: &str) -> Result<ConfigDocument> {
	let config_file_contents = fs::read_to_string(config_path).await.into_diagnostic()?;
	let config = knuffel::parse(config_path, &config_file_contents)?;
	Ok(config)
}

#[derive(Debug, Decode)]
pub struct ConfigDocument {
	#[knuffel(child)]
	pub openid: OpenIdConfig,
	#[knuffel(child, unwrap(argument))]
	pub session_secret_key_file: String,
	#[knuffel(child)]
	pub listen: ListenAddr,
	#[knuffel(child)]
	pub database: DatabaseArgs,
}

#[derive(Debug, Decode)]
pub struct OpenIdConfig {
	#[knuffel(child, unwrap(argument))]
	pub endpoint: String,
	#[knuffel(child, unwrap(argument))]
	pub client_id: String,
	#[knuffel(child, unwrap(argument))]
	pub secret: String,
	#[knuffel(child, unwrap(argument))]
	pub response_url: String,
	#[knuffel(child, unwrap(argument))]
	pub logout_url: String,
}

#[derive(Debug, Decode)]
pub struct ListenAddr {
	#[knuffel(argument)]
	pub addr: String,
}

#[derive(Debug, Decode)]
pub struct DatabaseArgs {
	#[knuffel(child, unwrap(argument))]
	pub host: String,
	#[knuffel(child, unwrap(argument))]
	pub port: Option<u16>,
	#[knuffel(child, unwrap(argument))]
	pub username: String,
	#[knuffel(child, unwrap(argument))]
	pub password: String,
	#[knuffel(child, unwrap(argument))]
	pub database: String,
}
