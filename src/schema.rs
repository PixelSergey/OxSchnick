// @generated automatically by Diesel CLI.

diesel::table! {
    schnicks (id) {
        id -> Int4,
        winner -> Int4,
        loser -> Int4,
        weapon -> Int4,
        played_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 32]
        username -> Nullable<Varchar>,
        parent -> Int4,
        #[max_length = 36]
        token -> Bpchar,
        #[max_length = 36]
        invite -> Bpchar,
        created -> Timestamptz,
        active -> Bool,
    }
}

diesel::allow_tables_to_appear_in_same_query!(schnicks, users,);
