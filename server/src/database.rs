use crate::config::ConfigDocument;
use async_std::sync::Arc;
use diesel::pg::PgConnection;
use diesel::Connection;
use miette::IntoDiagnostic;

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
