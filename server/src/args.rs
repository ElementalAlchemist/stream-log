use clap::Parser;

#[derive(Parser)]
#[command(name = "Stream Log")]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
	#[arg(
		long,
		help = "Only run database migrations to update the schema (don't start the web server)"
	)]
	pub migrations_only: bool,
}
