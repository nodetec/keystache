use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{Error, Value};

use nostr_sdk::prelude::*;

pub struct Database;

#[derive(Serialize, Deserialize)]
struct RegisterResponse {
    status: String,
    message: String,
}

impl RegisterResponse {
    fn success(message: String) -> Value {
        serde_json::to_value(RegisterResponse {
            status: "success".to_string(),
            message,
        })
        .unwrap()
    }

    fn error(message: String) -> Value {
        serde_json::to_value(RegisterResponse {
            status: "error".to_string(),
            message,
        })
        .unwrap()
    }
}

impl Database {
    pub fn register(nsec: String, npub: String) -> Value {
        let conn = match Connection::open("nostr_keys.db") {
            Ok(conn) => conn,
            Err(e) => return RegisterResponse::error(format!("Failed to open database: {}", e)),
        };

        if let Err(e) = conn.execute(
            "CREATE TABLE IF NOT EXISTS keys (
                id INTEGER PRIMARY KEY,
                npub TEXT NOT NULL,
                nsec TEXT NOT NULL,
                creation_date TEXT NOT NULL
            )",
            [],
        ) {
            return RegisterResponse::error(format!("Failed to create table: {}", e));
        }

        let creation_date = Utc::now().naive_utc();
        match conn.execute(
            "INSERT INTO keys (npub, nsec, creation_date) VALUES (?1, ?2, ?3)",
            params![
                npub,
                nsec,
                creation_date.format("%Y-%m-%d %H:%M:%S").to_string()
            ],
        ) {
            Ok(_) => RegisterResponse::success("Key registered successfully".to_string()),
            Err(e) => RegisterResponse::error(format!("Failed to insert key: {}", e)),
        }
    }

    pub fn get_nsec_by_npub(npub: &str) -> Result<String, String> {
        let conn = match Connection::open("nostr_keys.db") {
            Ok(conn) => conn,
            Err(e) => return Err(format!("Failed to open database: {}", e)),
        };

        let mut stmt = match conn.prepare("SELECT nsec FROM keys WHERE npub = ?1") {
            Ok(stmt) => stmt,
            Err(e) => return Err(format!("Failed to prepare query: {}", e)),
        };

        let nsec_iter = match stmt.query_map(params![npub], |row| row.get(0)) {
            Ok(iter) => iter,
            Err(e) => return Err(format!("Failed to execute query: {}", e)),
        };

        for nsec in nsec_iter {
            return nsec.map_err(|e| format!("Failed to fetch result: {}", e));
        }

        Err("No such public key found".to_string())
    }

    // TODO: we need to find a better way to do this
    pub fn get_first_nsec() -> Result<String, String> {
        let conn = match Connection::open("nostr_keys.db") {
            Ok(conn) => conn,
            Err(e) => return Err(format!("Failed to open database: {}", e)),
        };

        let mut stmt = match conn.prepare("SELECT nsec FROM keys ORDER BY id ASC LIMIT 1") {
            Ok(stmt) => stmt,
            Err(e) => return Err(format!("Failed to prepare query: {}", e)),
        };

        let nsec_iter = match stmt.query_map([], |row| row.get(0)) {
            Ok(iter) => iter,
            Err(e) => return Err(format!("Failed to execute query: {}", e)),
        };

        for nsec in nsec_iter {
            return nsec.map_err(|e| format!("Failed to fetch result: {}", e));
        }

        Err("No records found in the database".to_string())
    }
}
