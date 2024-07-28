// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::Parser;

#[derive(Parser)]
#[command(name = "Stream Log")]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
	#[arg(short, long, default_value = "config.kdl", help = "Configuration file path")]
	pub config: String,
	#[arg(
		long,
		help = "Only run database migrations to update the schema (don't start the web server)"
	)]
	pub migrations_only: bool,
}
