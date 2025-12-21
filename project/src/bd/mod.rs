use crate::globals::CONFIG;
use crate::model::transaction::TxId;
use crate::model::{Block, Transaction, TxOutput, UTXO};
use rusqlite::{Connection, Result, params};

pub struct Db {
    conn: Connection,
}

impl Db {
    // improve db interface
    pub fn open(path: Option<&str>) -> Result<Self> {
        let path = path.unwrap_or(&CONFIG.db_path);
        let db_path = std::path::Path::new(path);
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .expect("Failed to create directory for database file!");
            }
        }

        let conn = Connection::open(db_path)?;
        Ok(Db { conn })
    }

    pub fn get_conn(&self) -> &Connection {
        &self.conn
    }

    pub fn init_schema(&self) -> Result<()> {
        // Set pragmas for performance
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;",
        )?;

        // Create block_headers table
        self.conn.execute(
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

        // Create transactions table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS transactions (
                txid         BLOB PRIMARY KEY,
                raw          BLOB NOT NULL,
                block_hash   BLOB,
                block_height INTEGER,
                timestamp    INTEGER
            )",
            [],
        )?;

        // Create utxos table
        self.conn.execute(
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

        // Create tx_addresses table to track which addresses are used in transactions
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tx_addresses (
                txid BLOB NOT NULL,
                addr TEXT NOT NULL,
                PRIMARY KEY (txid, addr)
            ) WITHOUT ROWID",
            [],
        )?;

        // Create indices
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_utxos_addr ON utxos(addr)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tx_addresses_addr ON tx_addresses(addr)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tx_block_hash ON transactions(block_hash)",
            [],
        )?;

        Ok(())
    }

    pub fn get_utxos_for_address(&self, addr: &str) -> Result<Vec<UTXO>> {
        let mut stmt = self
            .conn
            .prepare("SELECT txid, vout, value, addr FROM utxos WHERE addr = ?1")?;

        let utxos = stmt.query_map([addr], |row| {
            let txid_vec: Vec<u8> = row.get(0)?;
            let mut txid = [0u8; 32];
            txid.copy_from_slice(&txid_vec);

            let vout: i64 = row.get(1)?;
            let value: i64 = row.get(2)?;
            let address: String = row.get(3)?;

            Ok(UTXO {
                tx_id: txid,
                index: vout as usize,
                output: TxOutput {
                    value: value as f64,
                    address,
                },
            })
        })?;

        let mut result = Vec::new();
        for utxo in utxos {
            result.push(utxo?);
        }
        Ok(result)
    }

    pub fn get_transaction(&self, txid: &[u8; 32]) -> Result<Option<Transaction>> {
        let mut stmt = self
            .conn
            .prepare("SELECT raw FROM transactions WHERE txid = ?1")?;

        let mut rows = stmt.query([txid.as_slice()])?;

        if let Some(row) = rows.next()? {
            let raw: Vec<u8> = row.get(0)?;
            let tx: Transaction = serde_json::from_slice(&raw)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(Some(tx))
        } else {
            Ok(None)
        }
    }

    pub fn apply_block(&mut self, block: Block, transactions: &[Transaction]) -> Result<()> {
        let tx = self.conn.transaction()?;

        let block_hash = block.header_hash();
        let header = &block.header;

        let height: i64 = if header.prev_block_hash == [0u8; 32] {
            0
        } else {
            let mut stmt = tx.prepare("SELECT height FROM block_headers WHERE block_hash = ?1")?;
            let mut rows = stmt.query([header.prev_block_hash.as_slice()])?;
            if let Some(row) = rows.next()? {
                let prev_height: i64 = row.get(0)?;
                prev_height + 1
            } else {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
        };

        let timestamp = header.timestamp.and_utc().timestamp();
        tx.execute(
            "INSERT INTO block_headers (block_hash, prev_hash, merkle_root, nonce, height, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                block_hash.as_slice(),
                header.prev_block_hash.as_slice(),
                header.merkle_root.as_slice(),
                header.nonce,
                height,
                timestamp
            ],
        )?;

        for transaction in transactions {
            let txid = transaction.id();
            let raw = serde_json::to_vec(transaction)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let tx_timestamp = transaction.date.and_utc().timestamp();

            // Insert transaction
            tx.execute(
                "INSERT OR REPLACE INTO transactions (txid, raw, block_hash, block_height, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    txid.as_slice(),
                    raw,
                    block_hash.as_slice(),
                    height,
                    tx_timestamp
                ],
            )?;

            // Remove spent UTXOs (inputs)
            for input in &transaction.inputs {
                tx.execute(
                    "DELETE FROM utxos WHERE txid = ?1 AND vout = ?2",
                    params![input.prev_tx_id.as_slice(), input.output_index as i64],
                )?;
            }

            // Add new UTXOs (outputs) and track addresses
            for (vout, output) in transaction.outputs.iter().enumerate() {
                tx.execute(
                    "INSERT INTO utxos (txid, vout, value, addr, script)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        txid.as_slice(),
                        vout as i64,
                        output.value as i64,
                        &output.address,
                        Vec::<u8>::new() // script placeholder
                    ],
                )?;

                // Track address usage in this transaction
                tx.execute(
                    "INSERT OR IGNORE INTO tx_addresses (txid, addr)
                     VALUES (?1, ?2)",
                    params![txid.as_slice(), &output.address],
                )?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_utxo(&self, txid: TxId, vout: usize) -> Result<Option<UTXO>> {
        let mut stmt = self
            .conn
            .prepare("SELECT txid, vout, value, addr FROM utxos WHERE txid = ?1 AND vout = ?2")?;

        let mut rows = stmt.query(params![txid.as_slice(), vout as i64])?;

        if let Some(row) = rows.next()? {
            let txid_vec: Vec<u8> = row.get(0)?;
            let mut txid_result = [0u8; 32];
            txid_result.copy_from_slice(&txid_vec);

            let vout_result: i64 = row.get(1)?;
            let value: i64 = row.get(2)?;
            let address: String = row.get(3)?;

            Ok(Some(UTXO {
                tx_id: txid_result,
                index: vout_result as usize,
                output: TxOutput {
                    value: value as f64,
                    address,
                },
            }))
        } else {
            Ok(None)
        }
    }

    pub fn insert_mempool_tx(&self, tx: &Transaction) -> Result<()> {
        let txid = tx.id();
        let raw = serde_json::to_vec(tx)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let timestamp = tx.date.and_utc().timestamp();

        self.conn.execute(
            "INSERT OR REPLACE INTO transactions (txid, raw, block_hash, block_height, timestamp)
             VALUES (?1, ?2, NULL, NULL, ?3)",
            params![txid.as_slice(), raw, timestamp],
        )?;

        Ok(())
    }

    pub fn remove_mempool_tx(&self, txid: &[u8; 32]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM transactions WHERE txid = ?1 AND block_hash IS NULL",
            [txid.as_slice()],
        )?;
        Ok(())
    }

    // Get all transactions that use a specific address
    pub fn get_transactions_for_address(&self, addr: &str) -> Result<Vec<[u8; 32]>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT txid FROM tx_addresses WHERE addr = ?1")?;

        let txids = stmt.query_map([addr], |row| {
            let txid_vec: Vec<u8> = row.get(0)?;
            let mut txid = [0u8; 32];
            txid.copy_from_slice(&txid_vec);
            Ok(txid)
        })?;

        let mut result = Vec::new();
        for txid in txids {
            result.push(txid?);
        }
        Ok(result)
    }

    // Check if an address has been used in any transaction
    pub fn has_address_been_used(&self, addr: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM tx_addresses WHERE addr = ?1 LIMIT 1")?;

        let has_address = stmt.exists([addr])?;
        Ok(has_address)
    }

    pub fn has_any_address_been_used(&self, addrs: &[String]) -> Result<bool> {
        println!("Checking addresses: {:?}", addrs);
        let mut query = String::from("SELECT 1 FROM tx_addresses WHERE addr IN (");
        let placeholders: Vec<String> = (0..addrs.len()).map(|_| "?".to_string()).collect();
        query.push_str(&placeholders.join(", "));
        query.push_str(") LIMIT 1");

        let mut stmt = self.conn.prepare(&query)?;

        let params: Vec<&dyn rusqlite::ToSql> =
            addrs.iter().map(|a| a as &dyn rusqlite::ToSql).collect();

        let has_address = stmt.exists(rusqlite::params_from_iter(params))?;
        Ok(has_address)
    }

    // Get all addresses used in a specific transaction
    pub fn get_addresses_in_transaction(&self, txid: &[u8; 32]) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT addr FROM tx_addresses WHERE txid = ?1")?;

        let addresses = stmt.query_map([txid.as_slice()], |row| row.get(0))?;

        let mut result = Vec::new();
        for addr in addresses {
            result.push(addr?);
        }
        Ok(result)
    }
}
