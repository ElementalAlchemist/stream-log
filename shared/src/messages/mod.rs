use serde::{Deserialize, Serialize};

pub mod initial;
pub mod user_register;

#[derive(Deserialize, Serialize)]
pub enum DataMessage<T> {
	Message(T),
	DatabaseError,
	ServerError
}