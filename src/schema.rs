// @generated automatically by Diesel CLI.

diesel::table! {
    calls (call_id) {
        call_id -> Int4,
        timestamp -> Nullable<Timestamp>,
        call_duration -> Nullable<Interval>,
        caller_id -> Nullable<Text>,
        caller_college -> Nullable<Int4>,
        time_on_hold -> Nullable<Interval>,
        question1 -> Nullable<Text>,
        answer1 -> Nullable<Text>,
        question2 -> Nullable<Text>,
        answer2 -> Nullable<Text>,
        question3 -> Nullable<Text>,
        answer3 -> Nullable<Text>,
        prophecy -> Nullable<Text>,
        language -> Nullable<Text>,
        match_id -> Nullable<Int4>,
        match_college -> Nullable<Int4>,
        feedback_path -> Nullable<Text>,
        completed -> Nullable<Bool>,
    }
}

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
    colleges (id) {
        id -> Int4,
        #[max_length = 32]
        college -> Varchar,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 32]
        username -> Varchar,
        college -> Nullable<Int4>,
        parent -> Int4,
        token -> Uuid,
        created -> Timestamptz,
        active -> Bool,
    }
}

diesel::joinable!(metrics -> users (id));
diesel::joinable!(users -> colleges (id));

diesel::allow_tables_to_appear_in_same_query!(calls, colleges, metrics, schnicks, users,);
