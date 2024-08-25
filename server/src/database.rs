// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::config::ConfigDocument;
use async_std::sync::Arc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use miette::{Diagnostic, IntoDiagnostic};
use r2d2::Error as R2D2Error;
use std::error::Error;
use std::fmt::{Debug, Display};
use tide::StatusCode;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

// To get boxed errors (as returned by the migration runner) into miette, we need a wrapper type for them.
#[derive(Debug, Diagnostic)]
pub struct MigrationError(pub Box<dyn Error + Send + Sync>);

impl Display for MigrationError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Display::fmt(&self.0, f)
	}
}

impl Error for MigrationError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		self.0.source()
	}
}

pub fn connect_db(config: &Arc<ConfigDocument>) -> miette::Result<Pool<ConnectionManager<PgConnection>>> {
	let url = db_url(config);
	let manager: ConnectionManager<PgConnection> = ConnectionManager::new(url);
	Pool::builder().test_on_check_out(true).build(manager).into_diagnostic()
}

fn db_url(config: &Arc<ConfigDocument>) -> String {
	if let Some(port) = config.database.port {
		format!(
			"postgres://{}:{}@{}:{}/{}",
			config.database.username, config.database.password, config.database.host, port, config.database.database
		)
	} else {
		format!(
			"postgres://{}:{}@{}/{}",
			config.database.username, config.database.password, config.database.host, config.database.database
		)
	}
}

pub fn run_embedded_migrations(
	db_connection_pool: &Pool<ConnectionManager<PgConnection>>,
) -> Result<(), MigrationError> {
	let mut db_connection = match db_connection_pool.get() {
		Ok(connection) => connection,
		Err(error) => return Err(MigrationError(Box::new(error))),
	};
	match db_connection.run_pending_migrations(MIGRATIONS) {
		Ok(_) => Ok(()),
		Err(error) => Err(MigrationError(error)),
	}
}

pub fn log_lost_db_connection(error: R2D2Error) {
	tide::log::error!("Database connection lost: {}", error);
}

pub fn handle_lost_db_connection(error: R2D2Error) -> tide::Result {
	log_lost_db_connection(error);
	Err(tide::Error::new(
		StatusCode::InternalServerError,
		anyhow::Error::msg("Couldn't connect to database"),
	))
}
