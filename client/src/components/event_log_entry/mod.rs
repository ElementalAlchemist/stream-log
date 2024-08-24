// © 2022-2024 Jacob Riddle (ElementalAlchemist)
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::subscriptions::event::TypingTarget;
use std::collections::HashMap;
use stream_log_shared::messages::user::PublicUserData;

pub mod edit;
pub mod entry;
pub mod row;
pub mod typing;
mod utils;

pub type UserTypingData = (PublicUserData, HashMap<TypingTarget, String>);
