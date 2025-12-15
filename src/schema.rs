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
    streaks (user_id) {
        user_id -> Int4,
        longest_winning_streak -> Int4,
        current_winning_streak -> Int4,
        longest_losing_streak -> Int4,
        current_losing_streak -> Int4,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 32]
        username -> Nullable<Varchar>,
        #[max_length = 4]
        dect -> Nullable<Bpchar>,
        parent -> Int4,
        #[max_length = 36]
        token -> Bpchar,
        created -> Timestamptz,
        active -> Bool,
    }
}

diesel::joinable!(streaks -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(schnicks, streaks, users,);
