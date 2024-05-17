use chrono::Utc;
use nostr_sdk::secp256k1::{Keypair, Secp256k1};
use nostr_sdk::{FromBech32, PublicKey, SecretKey, ToBech32};
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
    /// Creates a new database handle in the app's data directory.
    /// If an existing database is found, it will be opened.
    /// If the database does not exist, it will be created.
    ///
    /// # Arguments
    ///
    /// * `app_handle` - The Tauri application handle.
    /// * `encryption_key_or` - The encryption key for the database, or `None` if the database is not encrypted.
    ///                         If there is no existing database, the encryption key will be used to create a new encrypted database (if provided).
    ///                         If there is an existing database, the encryption key will be used to unlock the database (if provided) and an error will be returned if the key is incorrect.
    pub fn new_in_app_data_dir(
        app_handle: tauri::AppHandle,
        encryption_key_or: Option<&str>,
    ) -> anyhow::Result<Self> {
        let data_dir = match app_handle.path_resolver().app_data_dir() {
            Some(x) => x,
            None => return Err(anyhow::anyhow!("App data dir not found")),
        };

        Self::new(&data_dir, DATABASE_NAME, encryption_key_or)
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
                npub TEXT NOT NULL UNIQUE,
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

    /// Saves a keypair to the database.
    pub fn save_keypair(&self, keypair: &Keypair) -> anyhow::Result<()> {
        let db_connection = self.db_connection.lock().unwrap();

        let public_key: PublicKey = keypair.x_only_public_key().0.into();
        let secret_key: SecretKey = keypair.secret_key().into();

        db_connection.execute(
            "INSERT INTO keys (npub, nsec, create_time) VALUES (?1, ?2, ?3)",
            params![
                public_key.to_bech32()?,
                secret_key.to_bech32()?,
                Utc::now().to_rfc3339()
            ],
        )?;

        Ok(())
    }

    /// Removes a keypair from the database.
    /// If the keypair is associated with any registered applications, the
    /// caller must first unregister the applications or swap their
    /// application identities or an error will be returned.
    pub fn remove_keypair(&self, public_key: &PublicKey) -> anyhow::Result<()> {
        let db_connection = self.db_connection.lock().unwrap();

        db_connection.execute(
            "DELETE FROM keys WHERE npub = ?1",
            params![public_key.to_bech32()?],
        )?;

        Ok(())
    }

    /// Lists keypairs in the database. Ordered by id in ascending order.
    /// Use limit and offset parameters for pagination.
    pub fn list_keypairs(&self, limit: u64, offset: u64) -> anyhow::Result<Vec<Keypair>> {
        let db_connection = self.db_connection.lock().unwrap();

        let mut stmt =
            db_connection.prepare("SELECT nsec FROM keys ORDER BY id ASC LIMIT ?1 OFFSET ?2")?;

        let nsec_iter =
            stmt.query_map(params![limit, offset], |row| row.get::<usize, String>(0))?;

        let secp = Secp256k1::new();

        let mut keypairs = Vec::new();
        for nsec in nsec_iter {
            keypairs.push(SecretKey::from_bech32(nsec?)?.keypair(&secp));
        }

        Ok(keypairs)
    }

    /// Lists public keys of keypairs in the database. Ordered by id in ascending order.
    /// Use limit and offset parameters for pagination.
    pub fn list_public_keys(&self, limit: u64, offset: u64) -> anyhow::Result<Vec<PublicKey>> {
        let db_connection = self.db_connection.lock().unwrap();

        let mut stmt =
            db_connection.prepare("SELECT npub FROM keys ORDER BY id ASC LIMIT ?1 OFFSET ?2")?;

        let npub_iter =
            stmt.query_map(params![limit, offset], |row| row.get::<usize, String>(0))?;

        let mut npubs = Vec::new();
        for npub in npub_iter {
            npubs.push(PublicKey::from_bech32(npub?)?);
        }

        Ok(npubs)
    }

    /// Returns the first keypair in the database, or `None` if there are no keypairs.
    pub fn get_first_keypair(&self) -> anyhow::Result<Option<Keypair>> {
        Ok(self.list_keypairs(1, 0)?.first().cloned())
    }

    /// Returns the public key of the first keypair in the database, or `None` if there are no keypairs.
    pub fn get_first_public_key(&self) -> anyhow::Result<Option<PublicKey>> {
        Ok(self.list_public_keys(1, 0)?.first().cloned())
    }

    /// Adds a registered application to the database.
    pub fn register_application(
        &self,
        display_name: Option<String>,
        application_npub: &PublicKey,
        application_identity: &PublicKey,
    ) -> anyhow::Result<()> {
        let db_connection = self.db_connection.lock().unwrap();

        db_connection.execute(
            "INSERT INTO registered_applications (display_name, application_npub, create_time, application_identity) VALUES (?1, ?2, ?3, (SELECT id FROM keys WHERE npub = ?4))",
            params![display_name, application_npub.to_bech32()?, Utc::now().to_rfc3339(), application_identity.to_bech32()?],
        )?;

        Ok(())
    }

    /// Removes a registered application from the database.
    pub fn unregister_application(&self, application_npub: &PublicKey) -> anyhow::Result<()> {
        let db_connection = self.db_connection.lock().unwrap();

        db_connection.execute(
            "DELETE FROM registered_applications WHERE application_npub = ?1",
            params![application_npub.to_bech32()?],
        )?;

        Ok(())
    }

    /// Switches the keypair that a registered application is operating as.
    pub fn swap_application_identity(
        &self,
        application_npub: &PublicKey,
        new_application_identity: &PublicKey,
    ) -> anyhow::Result<()> {
        let db_connection = self.db_connection.lock().unwrap();

        let updated_row_count = db_connection.execute(
            "UPDATE registered_applications SET application_identity = (SELECT id FROM keys WHERE npub = ?1) WHERE application_npub = ?2",
            params![new_application_identity.to_bech32()?, application_npub.to_bech32()?],
        )?;

        if updated_row_count == 0 {
            return Err(anyhow::anyhow!(
                "Application with application_npub {} not found",
                application_npub.to_bech32()?
            ));
        }

        Ok(())
    }

    /// Lists registered applications in the database. Ordered by id in ascending order.
    /// Use limit and offset parameters for pagination.
    pub fn list_registered_applications(
        &self,
        limit: u64,
        offset: u64,
    ) -> anyhow::Result<Vec<(Option<String>, PublicKey, PublicKey)>> {
        let db_connection = self.db_connection.lock().unwrap();

        let mut stmt = db_connection.prepare(
            "SELECT display_name, application_npub, npub FROM registered_applications
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
            let (display_name, application_npub, application_identity_string) = application?;
            applications.push((
                display_name,
                PublicKey::from_bech32(application_npub)?,
                PublicKey::from_bech32(application_identity_string)?,
            ));
        }

        Ok(applications)
    }
}

#[cfg(test)]
mod tests {
    use nostr_sdk::secp256k1::rand::thread_rng;

    use super::*;
    use std::path::PathBuf;

    fn get_temp_folder() -> PathBuf {
        tempfile::TempDir::new()
            .expect("Failed to create temporary directory")
            .path()
            .to_path_buf()
    }

    fn get_random_keypair() -> Keypair {
        let secp = Secp256k1::new();
        Keypair::new(&secp, &mut thread_rng())
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
        assert!(Database::new(&folder.join("foo"), "test.db", None).is_err());
    }

    #[test]
    fn open_db_where_folder_exists_at_file_path() {
        let folder = get_temp_folder();

        std::fs::create_dir(&folder).unwrap();
        std::fs::create_dir(&folder.join("test.db")).unwrap();

        // Attempting to open a database where a folder already exists at the file path should cause an error.
        assert!(Database::new(&folder, "test.db", None).is_err());
    }

    #[test]
    fn reopen_unencrypted_db() {
        let folder = get_temp_folder();
        let db = Database::new(&folder, "test.db", None).unwrap();
        let keypair = get_random_keypair();
        db.save_keypair(&keypair).unwrap();

        drop(db);

        let db = Database::new(&folder, "test.db", None).unwrap();
        assert_eq!(db.get_first_keypair().unwrap().unwrap(), keypair);
    }

    #[test]
    fn reopen_encrypted_db() {
        let folder = get_temp_folder();
        let db = Database::new(&folder, "test.db", Some("hello world")).unwrap();
        let keypair = get_random_keypair();
        db.save_keypair(&keypair).unwrap();

        drop(db);

        let db = Database::new(&folder, "test.db", Some("hello world")).unwrap();
        assert_eq!(db.get_first_keypair().unwrap().unwrap(), keypair);
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
    fn save_and_remove_keypair() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();

        // Save a keypair to the database.
        db.save_keypair(&keypair).unwrap();

        // Check that the keypair was saved.
        assert_eq!(db.list_keypairs(1, 0).unwrap(), vec![keypair]);
        assert_eq!(
            db.list_public_keys(1, 0).unwrap(),
            vec![keypair.x_only_public_key().0.into()]
        );

        // Remove the keypair from the database.
        db.remove_keypair(&keypair.x_only_public_key().0.into())
            .unwrap();

        // Check that the keypair was removed.
        assert!(db.list_keypairs(1, 0).unwrap().is_empty());
        assert!(db.list_public_keys(1, 0).unwrap().is_empty());
    }

    #[test]
    fn save_duplicate_keypair() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();

        // Save a keypair to the database.
        db.save_keypair(&keypair).unwrap();

        // Saving the same keypair again should cause an error.
        assert!(db.save_keypair(&keypair).is_err());
    }

    #[test]
    fn remove_keypair_that_doesnt_exist() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();

        // Removing a keypair that doesn't exist should not cause an error.
        assert!(db
            .remove_keypair(&keypair.x_only_public_key().0.into())
            .is_ok());
    }

    #[test]
    fn list_keypairs() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Returns an empty list since there are no keypairs in the database.
        assert!(db.list_keypairs(10, 0).unwrap().is_empty());

        // Using an offset with an empty database should return an empty list.
        assert!(db.list_keypairs(10, 1).unwrap().is_empty());

        let keypair_1 = get_random_keypair();
        let keypair_2 = get_random_keypair();
        let keypair_3 = get_random_keypair();

        // Add some keypairs to the database.
        db.save_keypair(&keypair_1).unwrap();
        db.save_keypair(&keypair_2).unwrap();
        db.save_keypair(&keypair_3).unwrap();

        // Returns the keypairs in the database.
        assert_eq!(
            db.list_keypairs(10, 0).unwrap(),
            vec![keypair_1, keypair_2, keypair_3]
        );

        // Responds to limit.
        assert_eq!(db.list_keypairs(2, 0).unwrap(), vec![keypair_1, keypair_2]);

        // Responds to limit and offset.
        assert_eq!(db.list_keypairs(2, 2).unwrap(), vec![keypair_3]);

        // Limit of 0 should return an empty list.
        assert!(db.list_keypairs(0, 0).unwrap().is_empty());
    }

    #[test]
    fn list_public_keys() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Returns an empty list since there are no keypairs in the database.
        assert!(db.list_public_keys(10, 0).unwrap().is_empty());

        // Using an offset with an empty database should return an empty list.
        assert!(db.list_public_keys(10, 1).unwrap().is_empty());

        let keypair_1 = get_random_keypair();
        let keypair_2 = get_random_keypair();
        let keypair_3 = get_random_keypair();

        let pubkey_1 = keypair_1.x_only_public_key().0.into();
        let pubkey_2 = keypair_2.x_only_public_key().0.into();
        let pubkey_3 = keypair_3.x_only_public_key().0.into();

        // Add some keypairs to the database.
        db.save_keypair(&keypair_1).unwrap();
        db.save_keypair(&keypair_2).unwrap();
        db.save_keypair(&keypair_3).unwrap();

        // Returns the pubkeys in the database.
        assert_eq!(
            db.list_public_keys(10, 0).unwrap(),
            vec![pubkey_1, pubkey_2, pubkey_3]
        );

        // Responds to limit.
        assert_eq!(db.list_public_keys(2, 0).unwrap(), vec![pubkey_1, pubkey_2]);

        // Responds to limit and offset.
        assert_eq!(db.list_public_keys(2, 2).unwrap(), vec![pubkey_3]);

        // Limit of 0 should return an empty list.
        assert!(db.list_public_keys(0, 0).unwrap().is_empty());
    }

    #[test]
    fn get_first_keypair() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Returns `None` since there are no keypairs in the database.
        assert!(db.get_first_keypair().unwrap().is_none());

        let keypair_1 = get_random_keypair();
        let keypair_2 = get_random_keypair();
        let keypair_3 = get_random_keypair();

        // Add a keypair to the database.
        db.save_keypair(&keypair_1).unwrap();

        // Returns the newly added keypair.
        assert_eq!(db.get_first_keypair().unwrap(), Some(keypair_1));

        // Add more keypairs to the database.
        db.save_keypair(&keypair_2).unwrap();
        db.save_keypair(&keypair_3).unwrap();

        // Still returns the first keypair.
        assert_eq!(db.get_first_keypair().unwrap(), Some(keypair_1));

        // Remove the first keypair.
        db.remove_keypair(&keypair_1.x_only_public_key().0.into())
            .unwrap();

        // Returns the next keypair.
        assert_eq!(db.get_first_keypair().unwrap(), Some(keypair_2));
    }

    #[test]
    fn get_first_public_key() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();

        // Returns `None` since there are no keypairs in the database.
        assert!(db.get_first_public_key().unwrap().is_none());

        let keypair_1 = get_random_keypair();
        let keypair_2 = get_random_keypair();
        let keypair_3 = get_random_keypair();

        let pubkey_1 = keypair_1.x_only_public_key().0.into();
        let pubkey_2 = keypair_2.x_only_public_key().0.into();

        // Add a keypair to the database.
        db.save_keypair(&keypair_1).unwrap();

        // Returns the public key for the newly added keypair.
        assert_eq!(db.get_first_public_key().unwrap(), Some(pubkey_1));

        // Add more keypairs to the database.
        db.save_keypair(&keypair_2).unwrap();
        db.save_keypair(&keypair_3).unwrap();

        // Still returns the public key for the first keypair.
        assert_eq!(db.get_first_public_key().unwrap(), Some(pubkey_1));

        // Remove the first keypair.
        db.remove_keypair(&keypair_1.x_only_public_key().0.into())
            .unwrap();

        // Returns the public key for the next keypair.
        assert_eq!(db.get_first_public_key().unwrap(), Some(pubkey_2));
    }

    #[test]
    fn register_application_with_display_name_success() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();
        let application_keypair = get_random_keypair();

        db.save_keypair(&keypair).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            &application_keypair.x_only_public_key().0.into(),
            &keypair.x_only_public_key().0.into(),
        )
        .unwrap();
    }

    #[test]
    fn register_application_without_display_name_success() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();
        let application_keypair = get_random_keypair();

        db.save_keypair(&keypair).unwrap();

        db.register_application(
            None,
            &application_keypair.x_only_public_key().0.into(),
            &keypair.x_only_public_key().0.into(),
        )
        .unwrap();
    }

    #[test]
    fn register_multiple_applications_with_same_identity_success() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();
        let application_1_keypair = get_random_keypair();
        let application_2_keypair = get_random_keypair();

        db.save_keypair(&keypair).unwrap();

        db.register_application(
            Some("display_name1".to_string()),
            &application_1_keypair.x_only_public_key().0.into(),
            &keypair.x_only_public_key().0.into(),
        )
        .unwrap();
        db.register_application(
            Some("display_name2".to_string()),
            &application_2_keypair.x_only_public_key().0.into(),
            &keypair.x_only_public_key().0.into(),
        )
        .unwrap();
    }

    #[test]
    fn register_application_invalid_public_key_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();
        let application_keypair = get_random_keypair();

        // Attempting to register an application with a public key that doesn't exist should cause an error.
        let response = db.register_application(
            None,
            &application_keypair.x_only_public_key().0.into(),
            &keypair.x_only_public_key().0.into(),
        );
        assert!(response.is_err());
    }

    #[test]
    fn register_application_duplicate_application_npub_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();
        let application_keypair = get_random_keypair();

        db.save_keypair(&keypair).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            &application_keypair.x_only_public_key().0.into(),
            &keypair.x_only_public_key().0.into(),
        )
        .unwrap();

        // Attempting to register an application with the same application_npub should cause an error.
        let response = db.register_application(
            Some("display_name".to_string()),
            &application_keypair.x_only_public_key().0.into(),
            &keypair.x_only_public_key().0.into(),
        );
        assert!(response.is_err());
    }

    #[test]
    fn unregister_application_success() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();
        let application_keypair = get_random_keypair();

        db.save_keypair(&keypair).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            &application_keypair.x_only_public_key().0.into(),
            &keypair.x_only_public_key().0.into(),
        )
        .unwrap();

        db.unregister_application(&application_keypair.x_only_public_key().0.into())
            .unwrap();
    }

    #[test]
    fn unregister_application_invalid_application_npub_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let application_keypair = get_random_keypair();

        // Attempting to unregister an application with an application_npub that doesn't exist should not cause an error.
        let response = db.unregister_application(&application_keypair.x_only_public_key().0.into());
        assert!(response.is_ok());
    }

    #[test]
    fn swap_application_identity_success() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair_1 = get_random_keypair();
        let keypair_2 = get_random_keypair();
        let application_keypair = get_random_keypair();

        db.save_keypair(&keypair_1).unwrap();
        db.save_keypair(&keypair_2).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            &application_keypair.x_only_public_key().0.into(),
            &keypair_1.x_only_public_key().0.into(),
        )
        .unwrap();

        db.swap_application_identity(
            &application_keypair.x_only_public_key().0.into(),
            &keypair_2.x_only_public_key().0.into(),
        )
        .unwrap();
    }

    #[test]
    fn swap_application_identity_invalid_application_npub_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();
        let application_keypair = get_random_keypair();

        db.save_keypair(&keypair).unwrap();

        // Attempting to swap the application identity of an application with an application_npub that doesn't exist should cause an error.
        let response = db.swap_application_identity(
            &application_keypair.x_only_public_key().0.into(),
            &keypair.x_only_public_key().0.into(),
        );
        assert!(response.is_err());
    }

    #[test]
    fn swap_application_identity_invalid_new_identity_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair_1 = get_random_keypair();
        let keypair_2 = get_random_keypair();
        let application_keypair = get_random_keypair();

        // Only save the first keypair.
        db.save_keypair(&keypair_1).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            &application_keypair.x_only_public_key().0.into(),
            &keypair_1.x_only_public_key().0.into(),
        )
        .unwrap();

        // Attempting to swap the application identity of an application
        // with an npub that doesn't exist should cause an error.
        let response = db.swap_application_identity(
            &application_keypair.x_only_public_key().0.into(),
            &keypair_2.x_only_public_key().0.into(),
        );
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

        let keypair_1 = get_random_keypair();
        let keypair_2 = get_random_keypair();
        let keypair_3 = get_random_keypair();

        let application_keypair_1 = get_random_keypair();
        let application_keypair_2 = get_random_keypair();
        let application_keypair_3 = get_random_keypair();

        // Add some registered applications to the database.
        db.save_keypair(&keypair_1).unwrap();
        db.save_keypair(&keypair_2).unwrap();
        db.save_keypair(&keypair_3).unwrap();

        db.register_application(
            Some("display_name1".to_string()),
            &application_keypair_1.x_only_public_key().0.into(),
            &keypair_1.x_only_public_key().0.into(),
        )
        .unwrap();
        db.register_application(
            Some("display_name2".to_string()),
            &application_keypair_2.x_only_public_key().0.into(),
            &keypair_2.x_only_public_key().0.into(),
        )
        .unwrap();
        // Register an application without a display name.
        db.register_application(
            None,
            &application_keypair_3.x_only_public_key().0.into(),
            &keypair_3.x_only_public_key().0.into(),
        )
        .unwrap();

        // Returns the registered applications in the database.
        let response = db.list_registered_applications(10, 0).unwrap();
        assert_eq!(
            response,
            vec![
                (
                    Some("display_name1".to_string()),
                    application_keypair_1.x_only_public_key().0.into(),
                    keypair_1.x_only_public_key().0.into()
                ),
                (
                    Some("display_name2".to_string()),
                    application_keypair_2.x_only_public_key().0.into(),
                    keypair_2.x_only_public_key().0.into()
                ),
                (
                    None,
                    application_keypair_3.x_only_public_key().0.into(),
                    keypair_3.x_only_public_key().0.into()
                )
            ]
        );

        // Responds to limit.
        let response = db.list_registered_applications(2, 0).unwrap();
        assert_eq!(
            response,
            vec![
                (
                    Some("display_name1".to_string()),
                    application_keypair_1.x_only_public_key().0.into(),
                    keypair_1.x_only_public_key().0.into()
                ),
                (
                    Some("display_name2".to_string()),
                    application_keypair_2.x_only_public_key().0.into(),
                    keypair_2.x_only_public_key().0.into()
                )
            ]
        );

        // Responds to limit and offset.
        let response = db.list_registered_applications(2, 2).unwrap();
        assert_eq!(
            response,
            vec![(
                None,
                application_keypair_3.x_only_public_key().0.into(),
                keypair_3.x_only_public_key().0.into()
            )]
        );

        // Limit of 0 should return an empty list.
        let response = db.list_registered_applications(0, 0).unwrap();
        assert!(response.is_empty());
    }

    #[test]
    fn remove_keypair_used_by_registered_application_error() {
        let db = Database::new(&get_temp_folder(), "test.db", None).unwrap();
        let keypair = get_random_keypair();
        let application_keypair = get_random_keypair();

        db.save_keypair(&keypair).unwrap();

        db.register_application(
            Some("display_name".to_string()),
            &application_keypair.x_only_public_key().0.into(),
            &keypair.x_only_public_key().0.into(),
        )
        .unwrap();

        let response = db.remove_keypair(&keypair.x_only_public_key().0.into());
        assert!(response.is_err());

        // Unregister the application first, then remove the keypair.
        db.unregister_application(&application_keypair.x_only_public_key().0.into())
            .unwrap();
        db.remove_keypair(&keypair.x_only_public_key().0.into())
            .unwrap();
    }
}
