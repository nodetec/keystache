mod model;
mod schema;

use diesel::connection::SimpleConnection;
use diesel::delete;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{insert_into, prelude::*};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use model::{NewNostrKeypair, NostrKeypair};
use nostr_sdk::secp256k1::Keypair;
use nostr_sdk::{PublicKey, SecretKey, ToBech32};
use schema::nostr_keys::dsl::{id, nostr_keys, npub};
use std::path::Path;
use std::time::Duration;

const DATABASE_NAME: &str = "keystache.sqlite";
const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(Debug)]
struct ConnectionOptions {
    encryption_password: String,
    enable_wal: bool,
    enable_foreign_keys: bool,
    busy_timeout: Option<Duration>,
}

fn normalize_password(password: &str) -> String {
    password.replace('\'', "''")
}

impl diesel::r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error>
    for ConnectionOptions
{
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        (|| {
            let password = normalize_password(&self.encryption_password);
            conn.batch_execute(&format!("PRAGMA key='{password}'"))?;
            if self.enable_wal {
                conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
            }
            if self.enable_foreign_keys {
                conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            }
            if let Some(d) = self.busy_timeout {
                conn.batch_execute(&format!("PRAGMA busy_timeout = {};", d.as_millis()))?;
            }

            conn.run_pending_migrations(MIGRATIONS)
                .expect("Migration has to run successfully");

            Ok(())
        })()
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

// TODO: Handle database migrations.

/// Database handle for Keystache data.
#[derive(Clone)]
pub struct Database {
    connection_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl Database {
    // TODO: Test this.
    pub fn exists() -> bool {
        let project_dirs = Self::get_project_dirs().unwrap();
        let db_path = project_dirs.data_dir().join(DATABASE_NAME);
        db_path.is_file()
    }

    pub fn delete() {
        let project_dirs = Self::get_project_dirs().unwrap();
        let db_path = project_dirs.data_dir().join(DATABASE_NAME);
        std::fs::remove_file(db_path).unwrap();
    }

    /// Creates a new database handle in the app's data directory.
    /// If an existing database is found, it will be opened.
    /// If the database does not exist, it will be created.
    ///
    /// # Arguments
    ///
    /// * `encryption_password` - The encryption password for the database.
    ///                           If there is no existing database, the encryption password will be used to create a new encrypted database.
    ///                           If there is an existing database, the encryption password will be used to unlock the database and an error will be returned if the password is incorrect.
    pub fn open_or_create_in_app_data_dir(encryption_password: String) -> anyhow::Result<Self> {
        let project_dirs = Self::get_project_dirs()?;

        Self::open_or_create(project_dirs.data_dir(), DATABASE_NAME, encryption_password)
    }

    fn open_or_create(
        folder: &Path,
        file_name: &str,
        encryption_password: impl ToString,
    ) -> anyhow::Result<Self> {
        // TODO: See if this comment is still true and if the statement below is still needed.
        // The call to `ConnectionManager::new()` below doesn't
        // create the directory if it doesn't exist, so we
        // need to do it ourselves.
        if !folder.try_exists()? {
            std::fs::create_dir_all(folder)?;
        }

        let manager = ConnectionManager::<SqliteConnection>::new(
            folder.join(file_name).to_str().unwrap_or_default(),
        );

        let connection_pool = Pool::builder()
            .connection_customizer(Box::new(ConnectionOptions {
                encryption_password: encryption_password.to_string(),
                enable_wal: true,
                enable_foreign_keys: true,
                busy_timeout: Some(Duration::from_secs(15)),
            }))
            .build(manager)?;

        Ok(Self { connection_pool })
    }

    /// Saves a keypair to the database.
    pub fn save_keypair(&self, keypair: &Keypair) -> anyhow::Result<()> {
        let conn = &mut self.connection_pool.get()?;

        let public_key: PublicKey = keypair.x_only_public_key().0.into();
        let secret_key: SecretKey = keypair.secret_key().into();

        insert_into(schema::nostr_keys::table)
            .values(&NewNostrKeypair {
                display_name: None,
                npub: public_key.to_bech32()?,
                nsec: secret_key.to_bech32()?,
            })
            .execute(conn)?;

        Ok(())
    }

    /// Removes a keypair from the database.
    /// If the keypair is associated with any registered applications, the
    /// caller must first unregister the applications or swap their
    /// application identities or an error will be returned.
    pub fn remove_keypair(&self, public_key: &str) -> anyhow::Result<()> {
        let conn = &mut self.connection_pool.get()?;

        delete(nostr_keys.filter(npub.eq(public_key))).execute(conn)?;

        Ok(())
    }

    /// Lists keypairs in the database. Ordered by id in ascending order.
    /// Use limit and offset parameters for pagination.
    pub fn list_keypairs(&self, limit: i64, offset: i64) -> anyhow::Result<Vec<NostrKeypair>> {
        let conn = &mut self.connection_pool.get()?;

        Ok(nostr_keys
            .order(id)
            .limit(limit)
            .offset(offset)
            .load(conn)?)
    }

    /// Lists public keys of keypairs in the database. Ordered by id in ascending order.
    /// Use limit and offset parameters for pagination.
    pub fn list_public_keys(&self, limit: i64, offset: i64) -> anyhow::Result<Vec<String>> {
        let conn = &mut self.connection_pool.get()?;

        Ok(nostr_keys
            .select(npub)
            .order(id)
            .limit(limit)
            .offset(offset)
            .load(conn)?
            .into_iter()
            .collect())
    }

    fn get_project_dirs() -> anyhow::Result<directories::ProjectDirs> {
        directories::ProjectDirs::from("co", "nodetec", "keystache")
            .ok_or_else(|| anyhow::anyhow!("Could not determine Keystache project directories."))
    }
}

#[cfg(test)]
mod tests {
    use nostr_sdk::secp256k1::{rand::thread_rng, Secp256k1};

    use super::*;
    use std::path::PathBuf;

    const CORRECT_DB_KEY: &str = "correct_db_key";
    const INCORRECT_DB_KEY: &str = "incorrect_db_key";

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
        Database::open_or_create(&folder, "test.db", String::new()).unwrap();
    }

    #[test]
    fn open_db_where_folder_does_not_exist() {
        let folder = get_temp_folder();
        Database::open_or_create(
            &folder.join("non_existent_subfolder"),
            "test.db",
            String::new(),
        )
        .unwrap();
    }

    #[test]
    fn open_db_where_file_exists_at_folder_path() {
        let folder = get_temp_folder();

        std::fs::create_dir(&folder).unwrap();
        std::fs::File::create(&folder.join("foo")).unwrap();

        // Attempting to open a database where a file already exists at the folder path should cause an error.
        assert!(Database::open_or_create(&folder.join("foo"), "test.db", String::new()).is_err());
    }

    #[test]
    fn open_db_where_folder_exists_at_file_path() {
        let folder = get_temp_folder();

        std::fs::create_dir(&folder).unwrap();
        std::fs::create_dir(&folder.join("test.db")).unwrap();

        // Attempting to open a database where a folder already exists at the file path should cause an error.
        assert!(Database::open_or_create(&folder, "test.db", String::new()).is_err());
    }

    #[test]
    fn reopen_encrypted_db() {
        let folder = get_temp_folder();
        let db = Database::open_or_create(&folder, "test.db", CORRECT_DB_KEY).unwrap();
        let keypair = get_random_keypair();
        db.save_keypair(&keypair).unwrap();

        drop(db);

        let db = Database::open_or_create(&folder, "test.db", CORRECT_DB_KEY).unwrap();
        assert_eq!(db.get_first_keypair().unwrap().unwrap(), keypair);
    }

    #[test]
    fn reopen_encrypted_db_with_wrong_encryption_password_error() {
        let folder = get_temp_folder();
        let db = Database::open_or_create(&folder, "test.db", CORRECT_DB_KEY).unwrap();

        drop(db);

        let db = Database::open_or_create(&folder, "test.db", INCORRECT_DB_KEY);
        assert!(db.is_err());
    }

    #[test]
    fn save_and_remove_keypair() {
        let db = Database::open_or_create(&get_temp_folder(), "test.db", CORRECT_DB_KEY).unwrap();
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
        let db = Database::open_or_create(&get_temp_folder(), "test.db", CORRECT_DB_KEY).unwrap();
        let keypair = get_random_keypair();

        // Save a keypair to the database.
        db.save_keypair(&keypair).unwrap();

        // Saving the same keypair again should cause an error.
        assert!(db.save_keypair(&keypair).is_err());
    }

    #[test]
    fn remove_keypair_that_doesnt_exist() {
        let db = Database::open_or_create(&get_temp_folder(), "test.db", CORRECT_DB_KEY).unwrap();
        let keypair = get_random_keypair();

        // Removing a keypair that doesn't exist should not cause an error.
        assert!(db
            .remove_keypair(&keypair.x_only_public_key().0.into())
            .is_ok());
    }

    #[test]
    fn list_keypairs() {
        let db = Database::open_or_create(&get_temp_folder(), "test.db", CORRECT_DB_KEY).unwrap();

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
        let db = Database::open_or_create(&get_temp_folder(), "test.db", CORRECT_DB_KEY).unwrap();

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
}
