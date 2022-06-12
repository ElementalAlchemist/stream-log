table! {
	use crate::diesel_types::*;

	events (id) {
		id -> Text,
		name -> Text,
	}
}

table! {
	use crate::diesel_types::*;

	permission_events (permission_group, event) {
		permission_group -> Text,
		event -> Text,
		level -> Permission,
	}
}

table! {
	use crate::diesel_types::*;

	permission_groups (id) {
		id -> Text,
		name -> Text,
	}
}

table! {
	use crate::diesel_types::*;

	user_permissions (user_id, permission_group) {
		user_id -> Text,
		permission_group -> Text,
	}
}

table! {
	use crate::diesel_types::*;

	users (id) {
		id -> Text,
		google_user_id -> Text,
		name -> Text,
		is_admin -> Bool,
	}
}

joinable!(permission_events -> events (event));
joinable!(permission_events -> permission_groups (permission_group));
joinable!(user_permissions -> permission_groups (permission_group));
joinable!(user_permissions -> users (user_id));

allow_tables_to_appear_in_same_query!(events, permission_events, permission_groups, user_permissions, users,);
