use miette::Result;

mod config;
use config::parse_config;

fn main() -> Result<()> {
	let config = parse_config()?;
	println!("{:?}", config);
	Ok(())
}
