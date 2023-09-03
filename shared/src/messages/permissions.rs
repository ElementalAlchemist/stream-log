use serde::{Deserialize, Serialize};

/// Permission level available for sending over the socket
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PermissionLevel {
	View,
	Edit,
	Supervisor,
}

impl PermissionLevel {
	pub fn can_edit(&self) -> bool {
		matches!(self, Self::Supervisor | Self::Edit)
	}
}
