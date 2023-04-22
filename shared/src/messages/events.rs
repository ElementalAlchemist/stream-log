use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Event {
	pub id: String,
	pub name: String,
	pub start_time: DateTime<Utc>,
}
