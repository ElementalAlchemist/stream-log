table! {
	use crate::diesel_types::*;

	default_roles (event) {
		event -> Text,
		permission_level -> Permission,
	}
}

table! {
	use crate::diesel_types::*;

	events (id) {
		id -> Text,
		name -> Text,
	}
}

table! {
	use crate::diesel_types::*;

	roles (user_id, event) {
		user_id -> Text,
		event -> Text,
		permission_level -> Permission,
	}
}

table! {
	use crate::diesel_types::*;

	users (id) {
		id -> Text,
		google_user_id -> Text,
		name -> Text,
		account_level -> Approval,
	}
}

joinable!(default_roles -> events (event));
joinable!(roles -> events (event));
joinable!(roles -> users (user_id));

allow_tables_to_appear_in_same_query!(default_roles, events, roles, users,);
