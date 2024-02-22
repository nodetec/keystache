use std::sync::{Arc, Mutex};

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{Error, Value};

use nostr_sdk::prelude::*;

#[derive(Clone)]
pub struct Database {
    db_connection: Arc<Mutex<Connection>>,
}

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
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        let app_data_dir = app_handle.path_resolver().app_data_dir().unwrap();

        // Create app data directory. Toss any error, since it's ok if the directory already exists.
        let _ = std::fs::create_dir(&app_data_dir);

        let db_connection = match Connection::open(app_data_dir.join("nostr_keys.db")) {
            Ok(conn) => conn,
            Err(e) => panic!("Failed to open database: {}", e),
        };

        Database {
            db_connection: Arc::from(Mutex::from(db_connection)),
        }
    }

    pub fn register(&self, nsec: String, npub: String) -> Value {
        let db_connection = self.db_connection.lock().unwrap();

        if let Err(e) = db_connection.execute(
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
        match db_connection.execute(
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

    pub fn get_nsec_by_npub(&self, npub: &str) -> Result<String, String> {
        let db_connection = self.db_connection.lock().unwrap();

        let mut stmt = match db_connection.prepare("SELECT nsec FROM keys WHERE npub = ?1") {
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
    pub fn get_first_nsec(&self) -> Result<String, String> {
        let db_connection = self.db_connection.lock().unwrap();

        let mut stmt = match db_connection.prepare("SELECT nsec FROM keys ORDER BY id ASC LIMIT 1")
        {
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
