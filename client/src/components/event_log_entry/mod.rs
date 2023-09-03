use crate::subscriptions::event::TypingTarget;
use std::collections::HashMap;
use stream_log_shared::messages::user::UserData;

pub mod edit;
pub mod entry;
pub mod row;
pub mod typing;
mod utils;

pub type UserTypingData = (UserData, HashMap<TypingTarget, String>);
