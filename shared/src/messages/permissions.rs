use serde::{Deserialize, Serialize};

/// Permission level available for sending over the socket
#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum PermissionLevel {
	View,
	Edit,
}
