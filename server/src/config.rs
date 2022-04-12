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
	#[knuffel(child, unwrap(argument))]
	pub google_client_id: String,
	#[knuffel(child, unwrap(argument))]
	pub web_root_path: Option<String>,
	#[knuffel(child)]
	pub listen: ListenAddr,
}

#[derive(Debug, Decode)]
pub struct ListenAddr {
	#[knuffel(argument)]
	pub addr: String,
}
