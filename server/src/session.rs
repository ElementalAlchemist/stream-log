// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::models::Session as SessionDb;
use crate::schema::sessions;
use async_std::sync::{Arc, Mutex};
use diesel::prelude::*;
use tide::sessions::{Session, SessionStore};
use tide::utils::async_trait;

#[derive(Clone)]
pub struct DatabaseSessionStore {
	db_connection: Arc<Mutex<PgConnection>>,
}

impl DatabaseSessionStore {
	pub fn new(db_connection: Arc<Mutex<PgConnection>>) -> Self {
		Self { db_connection }
	}
}

impl std::fmt::Debug for DatabaseSessionStore {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "DatabaseSessionStore {{}}")
	}
}

#[async_trait]
impl SessionStore for DatabaseSessionStore {
	async fn load_session(&self, cookie_value: String) -> anyhow::Result<Option<Session>> {
		let mut db_connection = self.db_connection.lock().await;
		let session_id = Session::id_from_cookie_value(&cookie_value)?;
		let session: Option<SessionDb> = sessions::table
			.find(&session_id)
			.first(&mut *db_connection)
			.optional()?;
		match session {
			Some(session) => {
				let session_data: Session = serde_json::from_str(&session.data)?;
				Ok(Some(session_data))
			}
			None => Ok(None),
		}
	}

	async fn store_session(&self, session: Session) -> anyhow::Result<Option<String>> {
		let mut db_connection = self.db_connection.lock().await;
		let session_row: SessionDb = SessionDb {
			id: session.id().to_string(),
			data: serde_json::to_string(&session)?,
		};
		diesel::insert_into(sessions::table)
			.values(&session_row)
			.on_conflict(sessions::id)
			.do_update()
			.set(sessions::data.eq(&session_row.data))
			.execute(&mut *db_connection)?;
		Ok(session.into_cookie_value())
	}

	async fn destroy_session(&self, session: Session) -> anyhow::Result<()> {
		let mut db_connection = self.db_connection.lock().await;
		diesel::delete(sessions::table)
			.filter(sessions::id.eq(session.id()))
			.execute(&mut *db_connection)?;
		Ok(())
	}

	async fn clear_store(&self) -> anyhow::Result<()> {
		let mut db_connection = self.db_connection.lock().await;
		diesel::delete(sessions::table).execute(&mut *db_connection)?;
		Ok(())
	}
}
