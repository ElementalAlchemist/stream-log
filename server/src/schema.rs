// @generated automatically by Diesel CLI.

pub mod sql_types {
	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "permission"))]
	pub struct Permission;
}

diesel::table! {
	events (id) {
		id -> Text,
		name -> Text,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::Permission;

	permission_events (permission_group, event) {
		permission_group -> Text,
		event -> Text,
		level -> Permission,
	}
}

diesel::table! {
	permission_groups (id) {
		id -> Text,
		name -> Text,
	}
}

diesel::table! {
	user_permissions (user_id, permission_group) {
		user_id -> Text,
		permission_group -> Text,
	}
}

diesel::table! {
	users (id) {
		id -> Text,
		openid_user_id -> Text,
		name -> Text,
		is_admin -> Bool,
	}
}

diesel::joinable!(permission_events -> events (event));
diesel::joinable!(permission_events -> permission_groups (permission_group));
diesel::joinable!(user_permissions -> permission_groups (permission_group));
diesel::joinable!(user_permissions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(events, permission_events, permission_groups, user_permissions, users,);
