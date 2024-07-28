// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
