// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 32]
        username -> Nullable<Varchar>,
        parent -> Nullable<Int4>,
        #[max_length = 36]
        token -> Bpchar,
        #[max_length = 36]
        invite -> Bpchar,
    }
}
