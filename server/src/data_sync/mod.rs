mod admin;
pub mod connection;
mod event_selection;
mod register;

pub enum HandleConnectionError {
	ConnectionClosed,
	SendError(tide::Error),
}

impl From<tide::Error> for HandleConnectionError {
	fn from(error: tide::Error) -> Self {
		Self::SendError(error)
	}
}
