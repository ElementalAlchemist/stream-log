table! {
    use diesel::sql_types::*;
    use crate::models::schema::*;

    events (id) {
        id -> Text,
        name -> Text,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::models::schema::*;

    roles (user_id, event) {
        user_id -> Text,
        event -> Text,
        permission_level -> Permission,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::models::schema::*;

    users (id) {
        id -> Text,
        google_user_id -> Text,
        name -> Text,
    }
}

joinable!(roles -> events (event));
joinable!(roles -> users (user_id));

allow_tables_to_appear_in_same_query!(
    events,
    roles,
    users,
);
