#[derive(Clone, Copy, Debug)]
pub enum ConnectionState {
	Connected,
	Reconnecting,
	Lost,
}

impl Default for ConnectionState {
	fn default() -> Self {
		Self::Connected
	}
}
