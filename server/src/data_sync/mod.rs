pub mod connection;
mod register;
mod subscriptions;
mod user_profile;

pub enum HandleConnectionError {
	ConnectionClosed,
	SendError(tide::Error),
}

impl From<tide::Error> for HandleConnectionError {
	fn from(error: tide::Error) -> Self {
		Self::SendError(error)
	}
}
