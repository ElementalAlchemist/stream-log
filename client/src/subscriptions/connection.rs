// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
