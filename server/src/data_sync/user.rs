use crate::models::Permission;
use stream_log_shared::messages::events::Event;
use stream_log_shared::messages::user::UserData;

#[derive(Clone)]
pub enum UserDataUpdate {
	User(UserData),
	EventPermissions(Event, Option<Permission>),
}
