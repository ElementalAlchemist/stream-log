use knuffel::Decode;
use miette::{IntoDiagnostic, Result};
use std::fs;

pub fn parse_config() -> Result<ConfigDocument> {
	let config_file_contents = fs::read_to_string("config.kdl").into_diagnostic()?;
	let config = knuffel::parse("config.kdl", &config_file_contents)?;
	Ok(config)
}

#[derive(Debug, Decode)]
pub struct ConfigDocument {
	#[knuffel(child)]
	pub google_credentials: GoogleCredentials,
	#[knuffel(child, unwrap(argument))]
	pub session_secret_key_file: String,
	#[knuffel(child, unwrap(argument))]
	pub web_root_path: Option<String>,
	#[knuffel(child)]
	pub listen: ListenAddr,
	#[knuffel(child, unwrap(argument))]
	pub openid_response_url: String,
	#[knuffel(child)]
	pub database: DatabaseArgs,
}

#[derive(Debug, Decode)]
pub struct GoogleCredentials {
	#[knuffel(child, unwrap(argument))]
	pub client_id: String,
	#[knuffel(child, unwrap(argument))]
	pub secret: String,
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
