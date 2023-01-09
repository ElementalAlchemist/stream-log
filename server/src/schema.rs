// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "permission"))]
    pub struct Permission;
}

diesel::table! {
    available_event_types_for_event (event_type, event_id) {
        event_type -> Text,
        event_id -> Text,
    }
}

diesel::table! {
    event_types (id) {
        id -> Text,
        name -> Text,
        color_red -> Int4,
        color_green -> Int4,
        color_blue -> Int4,
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

diesel::joinable!(available_event_types_for_event -> event_types (event_type));
diesel::joinable!(available_event_types_for_event -> events (event_id));
diesel::joinable!(permission_events -> events (event));
diesel::joinable!(permission_events -> permission_groups (permission_group));
diesel::joinable!(user_permissions -> permission_groups (permission_group));
diesel::joinable!(user_permissions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    available_event_types_for_event,
    event_types,
    events,
    permission_events,
    permission_groups,
    user_permissions,
    users,
);
