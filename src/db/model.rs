// Diesel requires that these structs contain all fields in the table, even if they are not used.
#![allow(unused)]

use super::schema;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Insertable)]
#[diesel(table_name = schema::nostr_keys)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewNostrKeypair {
    pub display_name: Option<String>,
    pub npub: String,
    pub nsec: String,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = schema::nostr_keys)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NostrKeypair {
    pub id: i32,
    pub display_name: Option<String>,
    pub npub: String,
    pub nsec: String,
    pub create_time: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = schema::nostr_relays)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewNostrRelay {
    pub websocket_url: String,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = schema::nostr_relays)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NostrRelay {
    pub id: i32,
    pub websocket_url: String,
    pub create_time: NaiveDateTime,
}
