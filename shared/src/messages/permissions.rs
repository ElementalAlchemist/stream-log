use serde::{Deserialize, Serialize};

/// Permission level available for sending over the socket
#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub enum PermissionLevel {
	View,
	Edit,
}
