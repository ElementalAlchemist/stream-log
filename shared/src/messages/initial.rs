use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum InitialMessage {
	Welcome,
	Unauthorized(InitialMessageUnauthorized),
}

#[derive(Deserialize, Serialize)]
pub struct InitialMessageUnauthorized {
	google_client_id: String,
}

impl InitialMessageUnauthorized {
	pub fn new(google_client_id: String) -> Self {
		Self { google_client_id }
	}

	pub fn google_auth_client_id(&self) -> &str {
		&self.google_client_id
	}
}
