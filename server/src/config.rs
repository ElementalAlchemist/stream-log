use knuffel::Decode;
use miette::{IntoDiagnostic, Result};
use std::fs;

#[derive(Debug, Decode)]
pub struct ConfigDocument {
	#[knuffel(child, unwrap(argument))]
	google_client_id: String,
}

pub fn parse_config() -> Result<ConfigDocument> {
	let config_file_contents = fs::read_to_string("config.kdl").into_diagnostic()?;
	let config = knuffel::parse("config.kdl", &config_file_contents).into_diagnostic()?;
	Ok(config)
}