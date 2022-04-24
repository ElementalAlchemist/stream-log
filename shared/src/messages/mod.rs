use serde::{Deserialize, Serialize};

pub mod initial;
pub mod user_register;

#[derive(Deserialize, Serialize)]
pub enum DataError {
	DatabaseError,
	ServerError,
}

pub type DataMessage<T> = Result<T, DataError>;
