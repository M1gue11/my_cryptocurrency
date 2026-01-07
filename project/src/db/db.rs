use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Result;
use std::path::Path;

use crate::globals::CONFIG;

pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConnection = r2d2::PooledConnection<SqliteConnectionManager>;

#[derive(Clone)]
pub struct Db {
    pool: DbPool,
}
impl Db {
    pub fn open(path: Option<&str>) -> Result<Self> {
        let path_str = path.unwrap_or(&CONFIG.db_path);
        let db_path = Path::new(path_str);

        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .expect("Failed to create directory for database file!");
            }
        }

        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::builder()
            .max_size(5)
            .build(manager)
            .expect("Unable to create connection pool");

        let db = Db { pool };
        db.init_schema()?;
        Ok(db)
    }

    pub fn get_conn(&self) -> DbConnection {
        self.pool.get().expect("unable to get connection")
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.get_conn();

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS block_headers (
                block_hash   BLOB PRIMARY KEY,
                prev_hash    BLOB NOT NULL,
                merkle_root  BLOB NOT NULL,
                nonce        INTEGER,
                height       INTEGER NOT NULL,
                timestamp    INTEGER
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS transactions (
                txid         BLOB PRIMARY KEY,
                raw          BLOB NOT NULL,
                block_hash   BLOB,
                block_height INTEGER,
                timestamp    INTEGER
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS utxos (
                txid  BLOB NOT NULL,
                vout  INTEGER NOT NULL,
                value INTEGER NOT NULL,
                addr  TEXT,
                script BLOB,
                PRIMARY KEY (txid, vout)
            ) WITHOUT ROWID",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tx_addresses (
                txid BLOB NOT NULL,
                addr TEXT NOT NULL,
                PRIMARY KEY (txid, addr)
            ) WITHOUT ROWID",
            [],
        )?;

        // Indices for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_utxos_addr ON utxos(addr)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tx_addresses_addr ON tx_addresses(addr)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tx_block_hash ON transactions(block_hash)",
            [],
        )?;

        Ok(())
    }
}

static mut DB: Option<Db> = None;

pub fn init_db() {
    unsafe {
        DB = Some(Db::open(None).unwrap());
    }
}

// #[allow(static_mut_refs)]
// pub fn get_db_mut() -> &'static mut Db {
//     unsafe { DB.as_mut().expect("Node não inicializado") }
// }

#[allow(static_mut_refs)]
pub fn get_db() -> &'static Db {
    unsafe { DB.as_ref().expect("Db não inicializado") }
}
