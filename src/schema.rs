// @generated automatically by Diesel CLI.

diesel::table! {
    metrics (id) {
        id -> Int4,
        num_schnicks -> Int4,
        num_won -> Int4,
        longest_winning_streak -> Int4,
        current_winning_streak -> Int4,
        longest_losing_streak -> Int4,
        current_losing_streak -> Int4,
        num_children -> Int4,
        num_rock -> Int4,
        num_scissors -> Int4,
        num_paper -> Int4,
    }
}

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
        username -> Varchar,
        #[max_length = 4]
        dect -> Nullable<Bpchar>,
        parent -> Int4,
        token -> Uuid,
        created -> Timestamptz,
        active -> Bool,
    }
}

diesel::joinable!(metrics -> users (id));

diesel::allow_tables_to_appear_in_same_query!(metrics, schnicks, users,);
