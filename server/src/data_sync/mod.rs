pub mod connection;
mod register;
mod subscription_manager;
mod subscriptions;
mod user_profile;

pub use subscription_manager::SubscriptionManager;

pub enum HandleConnectionError {
	ConnectionClosed,
	SendError(tide::Error),
}

impl From<tide::Error> for HandleConnectionError {
	fn from(error: tide::Error) -> Self {
		Self::SendError(error)
	}
}
