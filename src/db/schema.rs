// @generated automatically by Diesel CLI.

diesel::table! {
    nostr_keys (id) {
        id -> Integer,
        display_name -> Nullable<Text>,
        npub -> Text,
        nsec -> Text,
        create_time -> Timestamp,
    }
}
