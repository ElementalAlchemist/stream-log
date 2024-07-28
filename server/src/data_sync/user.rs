// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::models::Permission;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::user::UserData;

#[derive(Clone)]
pub enum UserDataUpdate {
	User(UserData),
	EventPermissions(Event, Option<Permission>),
}
