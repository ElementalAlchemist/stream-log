use crate::config::ConfigDocument;
use async_std::sync::Arc;
use diesel::pg::PgConnection;
use diesel::Connection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use miette::{Diagnostic, IntoDiagnostic};
use std::error::Error;
use std::fmt::{Debug, Display};

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

pub fn connect_db(config: &Arc<ConfigDocument>) -> miette::Result<PgConnection> {
	let url = db_url(config);
	PgConnection::establish(&url).into_diagnostic()
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

pub fn run_embedded_migrations(db_connection: &mut PgConnection) -> Result<(), MigrationError> {
	match db_connection.run_pending_migrations(MIGRATIONS) {
		Ok(_) => Ok(()),
		Err(error) => Err(MigrationError(error)),
	}
}
