use crate::bd::repository::ledger::LedgerRepository;
use crate::globals::CONFIG;
use rusqlite::{Connection, OpenFlags, Result};

pub struct DbContext {
    write_conn: Connection,
    read_conn: Connection,
}

unsafe impl Send for DbContext {}
unsafe impl Sync for DbContext {}

impl DbContext {
    pub fn open(path: Option<&str>) -> Result<Self> {
        let path = path.unwrap_or(&CONFIG.db_path);
        let is_memory = path == ":memory:";
        let open_path = if is_memory {
            "file:shared_mem_db?mode=memory&cache=shared"
        } else {
            path
        };

        ensure_parent_dir(path, is_memory)?;

        let mut write_flags = OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_READ_WRITE;
        let mut read_flags = OpenFlags::SQLITE_OPEN_READ_ONLY;
        if is_memory {
            write_flags.insert(OpenFlags::SQLITE_OPEN_URI);
            read_flags.insert(OpenFlags::SQLITE_OPEN_URI);
        }

        let write_conn = Connection::open_with_flags(open_path, write_flags)?;

        let read_conn = Connection::open_with_flags(open_path, read_flags)?;

        let ctx = DbContext {
            write_conn,
            read_conn,
        };

        ctx.configure_write_pragmas()?;
        Ok(ctx)
    }

    pub fn init_schema(&self) -> Result<()> {
        self.write_conn.execute_batch(
            "PRAGMA journal_mode = WAL;\n             PRAGMA synchronous = NORMAL;\n             PRAGMA temp_store = MEMORY;",
        )?;

        self.write_conn.execute(
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

        self.write_conn.execute(
            "CREATE TABLE IF NOT EXISTS transactions (
                txid         BLOB PRIMARY KEY,
                raw          BLOB NOT NULL,
                block_hash   BLOB,
                block_height INTEGER,
                timestamp    INTEGER
            )",
            [],
        )?;

        self.write_conn.execute(
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

        self.write_conn.execute(
            "CREATE TABLE IF NOT EXISTS tx_addresses (
                txid BLOB NOT NULL,
                addr TEXT NOT NULL,
                PRIMARY KEY (txid, addr)
            ) WITHOUT ROWID",
            [],
        )?;

        self.write_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_utxos_addr ON utxos(addr)",
            [],
        )?;

        self.write_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tx_addresses_addr ON tx_addresses(addr)",
            [],
        )?;

        self.write_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tx_block_hash ON transactions(block_hash)",
            [],
        )?;

        Ok(())
    }

    pub fn ledger(&self) -> LedgerRepository {
        LedgerRepository::new(self.read_conn(), self.write_conn())
    }

    pub fn write_conn(&self) -> &Connection {
        &self.write_conn
    }

    pub fn read_conn(&self) -> &Connection {
        &self.read_conn
    }

    fn configure_write_pragmas(&self) -> Result<()> {
        self.write_conn.execute_batch(
            "PRAGMA journal_mode = WAL;\n             PRAGMA synchronous = NORMAL;\n             PRAGMA temp_store = MEMORY;",
        )?;
        Ok(())
    }
}

fn ensure_parent_dir(path: &str, is_memory: bool) -> Result<()> {
    if is_memory {
        return Ok(());
    }
    let db_path = std::path::Path::new(path);
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        }
    }
    Ok(())
}
