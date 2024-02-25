use chrono::Utc;
use nostr_sdk::prelude::*;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

const DATABASE_NAME: &str = "keystache.db";

// TODO: Handle database migrations.

/// Database handle for Keystache data.
#[derive(Clone)]
pub struct Database {
    db_connection: Arc<Mutex<Connection>>,
}

impl Database {
    /// Creates a new database handle in the Tauri app's data directory.
    pub fn new_in_app_data_dir(app_handle: tauri::AppHandle) -> anyhow::Result<Self> {
        let data_dir = match app_handle.path_resolver().app_data_dir() {
            Some(x) => x,
            None => return Err(anyhow::anyhow!("App data dir not found")),
        };

        Self::new(&data_dir, DATABASE_NAME, None)
    }

    fn new(
        folder: &Path,
        file_name: &str,
        encryption_key_or: Option<&str>,
    ) -> anyhow::Result<Self> {
        // The call to `Connection::open()` below doesn't
        // create the directory if it doesn't exist, so we
        // need to do it ourselves.
        if !folder.try_exists()? {
            std::fs::create_dir_all(folder)?;
        }

        let db_connection = Connection::open(folder.join(file_name))?;

        if let Some(encryption_key) = encryption_key_or {
            // Unlock the database with the encryption key.
            db_connection.pragma_update(None, "key", encryption_key)?;
        }

        db_connection.execute(
            "CREATE TABLE IF NOT EXISTS keys (
                id INTEGER PRIMARY KEY,
                nsec TEXT NOT NULL UNIQUE,
                create_time TEXT NOT NULL
            )",
            [],
        )?;

        db_connection.execute(
            "CREATE TABLE IF NOT EXISTS registered_applications (
                id INTEGER PRIMARY KEY,
                display_name TEXT,
                application_npub TEXT NOT NULL UNIQUE,
                create_time TEXT NOT NULL,
                application_identity INTEGER NOT NULL,
                FOREIGN KEY (application_identity) REFERENCES keys(id)
            )",
            [],
        )?;

        Ok(Database {
            db_connection: Arc::from(Mutex::from(db_connection)),
        })
    }

    /// Saves an nsec to the database.
    pub fn save_nsec(&self, nsec: String) -> Result<(), rusqlite::Error> {
        let db_connection = self.db_connection.lock().unwrap();

        db_connection.execute(
            "INSERT INTO keys (nsec, create_time) VALUES (?1, ?2)",
            params![nsec, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Removes an nsec from the database.
    /// Returns an error if the nsec doesn't exist or if it is associated with any registered applications.
    /// If the nsec is associated with any registered applications, the caller should unregister the applications first.
    pub fn remove_nsec(&self, nsec: &str) -> Result<(), rusqlite::Error> {
        let db_connection = self.db_connection.lock().unwrap();

        db_connection.execute("DELETE FROM keys WHERE nsec = ?1", params![nsec])?;
        Ok(())
    }

    /// Lists nsecs in the database. Ordered by id in ascending order.
    /// Use limit and offset parameters for pagination.
    pub fn list_nsecs(&self, limit: u64, offset: u64) -> Result<Vec<String>, rusqlite::Error> {
        let db_connection = self.db_connection.lock().unwrap();

        let mut stmt =
            db_connection.prepare("SELECT nsec FROM keys ORDER BY id ASC LIMIT ?1 OFFSET ?2")?;

        let nsec_iter = stmt.query_map(params![limit, offset], |row| row.get(0))?;

        let mut nsecs = Vec::new();
        for nsec in nsec_iter {
            nsecs.push(nsec?);
        }

        Ok(nsecs)
    }

    /// Returns the first nsec in the database, or `None` if there are no nsecs.
    pub fn get_first_nsec(&self) -> Result<Option<String>, rusqlite::Error> {
        Ok(self.list_nsecs(1, 0)?.first().cloned())
    }

    /// Add a registered application to the database.
    pub fn register_application(
        &self,
        display_name: Option<String>,
        application_npub: String,
        application_identity_nsec: String, // TODO: Make this take an npub instead of an nsec.
    ) -> Result<(), rusqlite::Error> {
        let db_connection = self.db_connection.lock().unwrap();

        db_connection.execute(
            "INSERT INTO registered_applications (display_name, application_npub, create_time, application_identity) VALUES (?1, ?2, ?3, (SELECT id FROM keys WHERE nsec = ?4))",
            params![display_name, application_npub, Utc::now().to_rfc3339(), application_identity_nsec],
        )?;
        Ok(())
    }

    /// Removes a registered application from the database.
    pub fn unregister_application(&self, application_npub: &str) -> Result<(), rusqlite::Error> {
        let db_connection = self.db_connection.lock().unwrap();

        db_connection.execute(
            "DELETE FROM registered_applications WHERE application_npub = ?1",
            params![application_npub],
        )?;
        Ok(())
    }

    /// Switch the Nostr key pair that a registered application is operating as.
    pub fn swap_application_identity(
        &self,
        application_npub: &str,
        new_application_identity_nsec: &str,
    ) -> Result<(), rusqlite::Error> {
        let db_connection = self.db_connection.lock().unwrap();

        let updated_row_count = db_connection.execute(
            "UPDATE registered_applications SET application_identity = (SELECT id FROM keys WHERE nsec = ?1) WHERE application_npub = ?2",
            params![new_application_identity_nsec, application_npub],
        )?;
        if updated_row_count == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    /// Lists registered applications in the database. Ordered by id in ascending order.
    /// Use limit and offset parameters for pagination.
    pub fn list_registered_applications(
        &self,
        limit: u64,
        offset: u64,
    ) -> Result<Vec<(Option<String>, String, String)>, rusqlite::Error> {
        let db_connection = self.db_connection.lock().unwrap();

        let mut stmt = db_connection.prepare(
            "SELECT display_name, application_npub, nsec FROM registered_applications
            INNER JOIN keys ON registered_applications.application_identity = keys.id
            ORDER BY registered_applications.id ASC LIMIT ?1 OFFSET ?2",
        )?;

        let application_iter = stmt.query_map(params![limit, offset], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut applications = Vec::new();
        for application in application_iter {
            applications.push(application?);
        }

        Ok(applications)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_temp_folder() -> PathBuf {
        tempfile::TempDir::new()
            .expect("Failed to create temporary directory")
            .path()
            .to_path_buf()
    }

    #[test]
    fn open_db_where_folder_exists() {
        let folder = get_temp_folder();
        Database::new(&folder, "test.db", None).unwrap();
    }

    #[test]
    fn open_db_where_folder_does_not_exist() {
        let folder = get_temp_folder();
        Database::new(&folder.join("non_existent_subfolder"), "test.db", None).unwrap();
    }

    #[test]
    fn open_db_where_file_exists_at_folder_path() {
        let folder = get_temp_folder();

        std::fs::create_dir(&folder).unwrap();
        std::fs::File::create(&folder.join("foo")).unwrap();

        // Attempting to open a database where a file already exists at the folder path should cause an error.
        let response = Database::new(&folder.join("foo"), "test.db", None);
        assert!(response.is_err());
    }

    #[test]
    fn open_db_where_folder_exists_at_file_path() {
        let folder = get_temp_folder();

        std::fs::create_dir(&folder).unwrap();
        std::fs::create_dir(&folder.join("test.db")).unwrap();

        // Attempting to open a database where a folder already exists at the file path should cause an error.
        let response = Database::new(&folder, "test.db", None);
        assert!(response.is_err());
    }

    #[test]
    fn reopen_unencrypted_db() {
        let folder = get_temp_folder();
        let db = Database::new(&folder, "test.db", None).unwrap();
        db.save_nsec("nsec".to_string()).unwrap();

        drop(db);

        let db = Database::new(&folder, "test.db", None).unwrap();
        let response = db.get_first_nsec().unwrap().unwrap();
        assert_eq!(response, "nsec");
    }

    #[test]
    fn reopen_encrypted_db() {
        let folder = get_temp_folder();
        let db = Database::new(&folder, "test.db", Some("hello world")).unwrap();
        db.save_nsec("nsec".to_string()).unwrap();

        drop(db);

        let db = Database::new(&folder, "test.db", Some("hello world")).unwrap();
        let response = db.get_first_nsec().unwrap().unwrap();
        assert_eq!(response, "nsec");
    }

    #[test]
    fn reopen_encrypted_db_with_wrong_encryption_key_error() {
        let folder = get_temp_folder();
        let db = Database::new(&folder, "test.db", Some("hello world")).unwrap();

        drop(db);

        let db = Database::new(&folder, "test.db", Some("wrong key"));
        assert!(db.is_err());
    }

    #[test]
    fn reopen_encrypted_db_with_no_encryption_key_error() {
        let folder = get_temp_folder();
        let db = Database::new(&folder, "test.db", Some("hello world")).unwrap();

        drop(db);

        let db = Database::new(&folder, "test.db", None);
        assert!(db.is_err());
    }

    #[test]
    fn reopen_unencrypted_db_with_encryption_key_error() {
        let folder = get_temp_folder();
        let db = Database::new(&folder, "test.db", None).unwrap();

        drop(db);

        let db = Database::new(&folder, "test.db", Some("hello world"));
        assert!(db.is_err());
    }

    #[test]
    fn save_and_remove_nsec() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Save an nsec to the database.
        db.save_nsec("nsec1".to_string()).unwrap();

        // Check that the nsec was saved.
        let response = db.list_nsecs(1, 0).unwrap();
        assert_eq!(response, vec!["nsec1".to_string()]);

        // Remove the nsec from the database.
        db.remove_nsec("nsec1").unwrap();

        // Check that the nsec was removed.
        let response = db.list_nsecs(1, 0).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn save_duplicate_nsec() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Save an nsec to the database.
        db.save_nsec("nsec1".to_string()).unwrap();

        // Saving the same nsec again should cause an error.
        let response = db.save_nsec("nsec1".to_string());
        assert!(response.is_err());
    }

    #[test]
    fn remove_nsec_that_doesnt_exist() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Removing an nsec that doesn't exist should not cause an error.
        let response = db.remove_nsec("nsec1");
        assert!(response.is_ok());
    }

    #[test]
    fn list_nsecs() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Returns an empty list since there are no nsecs in the database.
        let response = db.list_nsecs(10, 0).unwrap();
        assert!(response.is_empty());

        // Using an offset with an empty database should return an empty list.
        let response = db.list_nsecs(10, 1).unwrap();
        assert!(response.is_empty());

        // Add some nsecs to the database.
        db.save_nsec("nsec1".to_string()).unwrap();
        db.save_nsec("nsec2".to_string()).unwrap();
        db.save_nsec("nsec3".to_string()).unwrap();

        // Returns the nsecs in the database.
        let response = db.list_nsecs(10, 0).unwrap();
        assert_eq!(
            response,
            vec![
                "nsec1".to_string(),
                "nsec2".to_string(),
                "nsec3".to_string()
            ]
        );

        // Responds to limit.
        let response = db.list_nsecs(2, 0).unwrap();
        assert_eq!(response, vec!["nsec1".to_string(), "nsec2".to_string()]);

        // Responds to limit and offset.
        let response = db.list_nsecs(2, 2).unwrap();
        assert_eq!(response, vec!["nsec3".to_string()]);

        // Limit of 0 should return an empty list.
        let response = db.list_nsecs(0, 0).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn get_first_nsec() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Returns `None` since there are no nsecs in the database.
        let response = db.get_first_nsec().unwrap();
        assert_eq!(response, None);

        // Add an nsec to the database.
        db.save_nsec("nsec1".to_string()).unwrap();

        // Returns the newly added nsec.
        let response = db.get_first_nsec().unwrap();
        assert_eq!(response, Some("nsec1".to_string()));

        // Add more nsecs to the database.
        db.save_nsec("nsec2".to_string()).unwrap();
        db.save_nsec("nsec3".to_string()).unwrap();

        // Still returns the first nsec.
        let response = db.get_first_nsec().unwrap();
        assert_eq!(response, Some("nsec1".to_string()));

        // Remove the first nsec.
        db.remove_nsec("nsec1").unwrap();

        // Returns the next nsec.
        let response = db.get_first_nsec().unwrap();
        assert_eq!(response, Some("nsec2".to_string()));
    }

    #[test]
    fn register_application_with_display_name_success() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        db.save_nsec("nsec".to_string()).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            "application_npub".to_string(),
            "nsec".to_string(),
        )
        .unwrap();
    }

    #[test]
    fn register_application_without_display_name_success() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        db.save_nsec("nsec".to_string()).unwrap();

        db.register_application(None, "application_npub".to_string(), "nsec".to_string())
            .unwrap();
    }

    #[test]
    fn register_multiple_applications_with_same_identity_success() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        db.save_nsec("nsec".to_string()).unwrap();

        db.register_application(
            Some("display_name1".to_string()),
            "application_npub1".to_string(),
            "nsec".to_string(),
        )
        .unwrap();
        db.register_application(
            Some("display_name2".to_string()),
            "application_npub2".to_string(),
            "nsec".to_string(),
        )
        .unwrap();
    }

    #[test]
    fn register_application_invalid_nsec_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Attempting to register an application with an nsec that doesn't exist should cause an error.
        let response = db.register_application(
            Some("display_name".to_string()),
            "application_npub".to_string(),
            "nsec".to_string(),
        );
        assert!(response.is_err());
    }

    #[test]
    fn register_application_duplicate_application_npub_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        db.save_nsec("nsec1".to_string()).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            "application_npub".to_string(),
            "nsec1".to_string(),
        )
        .unwrap();

        // Attempting to register an application with the same application_npub should cause an error.
        let response = db.register_application(
            Some("display_name".to_string()),
            "application_npub".to_string(),
            "nsec2".to_string(),
        );
        assert!(response.is_err());
    }

    #[test]
    fn unregister_application_success() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        db.save_nsec("nsec".to_string()).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            "application_npub".to_string(),
            "nsec".to_string(),
        )
        .unwrap();

        db.unregister_application("application_npub").unwrap();
    }

    #[test]
    fn unregister_application_invalid_application_npub_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Attempting to unregister an application with an application_npub that doesn't exist should not cause an error.
        let response = db.unregister_application("application_npub");
        assert!(response.is_ok());
    }

    #[test]
    fn swap_application_identity_success() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        db.save_nsec("nsec1".to_string()).unwrap();
        db.save_nsec("nsec2".to_string()).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            "application_npub".to_string(),
            "nsec1".to_string(),
        )
        .unwrap();

        db.swap_application_identity("application_npub", "nsec2")
            .unwrap();
    }

    #[test]
    fn swap_application_identity_invalid_application_npub_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        db.save_nsec("nsec1".to_string()).unwrap();
        db.save_nsec("nsec2".to_string()).unwrap();

        // Attempting to swap the application identity of an application with an application_npub that doesn't exist should cause an error.
        let response = db.swap_application_identity("application_npub", "nsec2");
        assert!(response.is_err());
    }

    #[test]
    fn swap_application_identity_invalid_new_identity_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        db.save_nsec("nsec1".to_string()).unwrap();
        db.save_nsec("nsec2".to_string()).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            "application_npub".to_string(),
            "nsec1".to_string(),
        )
        .unwrap();

        // Attempting to swap the application identity of an application with an nsec that doesn't exist should cause an error.
        let response = db.swap_application_identity("application_npub", "nsec3");
        assert!(response.is_err());
    }

    #[test]
    fn list_registered_applications() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Returns an empty list since there are no registered applications in the database.
        let response = db.list_registered_applications(10, 0).unwrap();
        assert!(response.is_empty());

        // Using an offset with an empty database should return an empty list.
        let response = db.list_registered_applications(10, 1).unwrap();
        assert!(response.is_empty());

        // Add some registered applications to the database.
        db.save_nsec("nsec1".to_string()).unwrap();
        db.save_nsec("nsec2".to_string()).unwrap();
        db.save_nsec("nsec3".to_string()).unwrap();

        db.register_application(
            Some("display_name1".to_string()),
            "application_npub1".to_string(),
            "nsec1".to_string(),
        )
        .unwrap();
        db.register_application(
            Some("display_name2".to_string()),
            "application_npub2".to_string(),
            "nsec2".to_string(),
        )
        .unwrap();
        // Register an application without a display name.
        db.register_application(None, "application_npub3".to_string(), "nsec3".to_string())
            .unwrap();

        // Returns the registered applications in the database.
        let response = db.list_registered_applications(10, 0).unwrap();
        assert_eq!(
            response,
            vec![
                (
                    Some("display_name1".to_string()),
                    "application_npub1".to_string(),
                    "nsec1".to_string()
                ),
                (
                    Some("display_name2".to_string()),
                    "application_npub2".to_string(),
                    "nsec2".to_string()
                ),
                (None, "application_npub3".to_string(), "nsec3".to_string())
            ]
        );

        // Responds to limit.
        let response = db.list_registered_applications(2, 0).unwrap();
        assert_eq!(
            response,
            vec![
                (
                    Some("display_name1".to_string()),
                    "application_npub1".to_string(),
                    "nsec1".to_string()
                ),
                (
                    Some("display_name2".to_string()),
                    "application_npub2".to_string(),
                    "nsec2".to_string()
                )
            ]
        );

        // Responds to limit and offset.
        let response = db.list_registered_applications(2, 2).unwrap();
        assert_eq!(
            response,
            vec![(None, "application_npub3".to_string(), "nsec3".to_string())]
        );

        // Limit of 0 should return an empty list.
        let response = db.list_registered_applications(0, 0).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn remove_nsec_used_by_registered_application_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        db.save_nsec("nsec".to_string()).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            "application_npub".to_string(),
            "nsec".to_string(),
        )
        .unwrap();

        let response = db.remove_nsec("nsec");
        assert!(response.is_err());

        // Unregister the application first, then remove the nsec.
        db.unregister_application("application_npub").unwrap();
        db.remove_nsec("nsec").unwrap();
    }
}
