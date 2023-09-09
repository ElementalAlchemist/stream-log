// @generated automatically by Diesel CLI.

pub mod sql_types {
	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "permission"))]
	pub struct Permission;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "video_edit_state"))]
	pub struct VideoEditState;

	#[derive(diesel::sql_types::SqlType)]
	#[diesel(postgres_type(name = "video_state"))]
	pub struct VideoState;
}

diesel::table! {
	applications (id) {
		id -> Text,
		name -> Text,
		auth_key -> Nullable<Text>,
		read_log -> Bool,
		write_links -> Bool,
		creation_user -> Text,
	}
}

diesel::table! {
	available_entry_types_for_event (entry_type, event_id) {
		entry_type -> Text,
		event_id -> Text,
	}
}

diesel::table! {
	entry_types (id) {
		id -> Text,
		name -> Text,
		color_red -> Int4,
		color_green -> Int4,
		color_blue -> Int4,
		description -> Text,
	}
}

diesel::table! {
	event_editors (event, editor) {
		event -> Text,
		editor -> Text,
	}
}

diesel::table! {
	use diesel::sql_types::*;
	use super::sql_types::VideoState;
	use super::sql_types::VideoEditState;

	event_log (id) {
		id -> Text,
		event -> Text,
		start_time -> Timestamptz,
		end_time -> Nullable<Timestamptz>,
		entry_type -> Text,
		description -> Text,
		media_link -> Text,
		submitter_or_winner -> Text,
		notes_to_editor -> Text,
		editor_link -> Nullable<Text>,
		editor -> Nullable<Text>,
		video_link -> Nullable<Text>,
		last_updated -> Timestamptz,
		last_update_user -> Text,
		parent -> Nullable<Text>,
		deleted_by -> Nullable<Text>,
		created_at -> Timestamptz,
		manual_sort_key -> Nullable<Int4>,
		video_state -> Nullable<VideoState>,
		video_errors -> Text,
		poster_moment -> Bool,
		video_edit_state -> VideoEditState,
		marked_incomplete -> Bool,
	}
}

diesel::table! {
	event_log_sections (id) {
		id -> Text,
		event -> Text,
		name -> Text,
		start_time -> Timestamptz,
	}
}

diesel::table! {
	event_log_tags (tag, log_entry) {
		tag -> Text,
		log_entry -> Text,
	}
}

diesel::table! {
	events (id) {
		id -> Text,
		name -> Text,
		start_time -> Timestamptz,
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
	tags (id) {
		id -> Text,
		tag -> Text,
		description -> Text,
		playlist -> Text,
		for_event -> Text,
		deleted -> Bool,
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
		color_red -> Int4,
		color_green -> Int4,
		color_blue -> Int4,
	}
}

diesel::joinable!(applications -> users (creation_user));
diesel::joinable!(available_entry_types_for_event -> entry_types (entry_type));
diesel::joinable!(available_entry_types_for_event -> events (event_id));
diesel::joinable!(event_editors -> events (event));
diesel::joinable!(event_editors -> users (editor));
diesel::joinable!(event_log -> entry_types (entry_type));
diesel::joinable!(event_log -> events (event));
diesel::joinable!(event_log_sections -> events (event));
diesel::joinable!(event_log_tags -> event_log (log_entry));
diesel::joinable!(event_log_tags -> tags (tag));
diesel::joinable!(permission_events -> events (event));
diesel::joinable!(permission_events -> permission_groups (permission_group));
diesel::joinable!(tags -> events (for_event));
diesel::joinable!(user_permissions -> permission_groups (permission_group));
diesel::joinable!(user_permissions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
	applications,
	available_entry_types_for_event,
	entry_types,
	event_editors,
	event_log,
	event_log_sections,
	event_log_tags,
	events,
	permission_events,
	permission_groups,
	tags,
	user_permissions,
	users,
);
