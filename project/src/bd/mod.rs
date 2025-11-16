use rusqlite::{Connection, Result, params};
use crate::model::{Transaction, TxOutput, UTXO};
use crate::model::block::BlockHeader;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Db { conn })
    }

    pub fn init_schema(&self) -> Result<()> {
        // Set pragmas for performance
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;"
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

        // Create indices
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_utxos_addr ON utxos(addr)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tx_block_hash ON transactions(block_hash)",
            [],
        )?;

        Ok(())
    }

    pub fn get_utxos_for_address(&self, addr: &str) -> Result<Vec<UTXO>> {
        let mut stmt = self.conn.prepare(
            "SELECT txid, vout, value, addr FROM utxos WHERE addr = ?1"
        )?;

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
        let mut stmt = self.conn.prepare(
            "SELECT raw FROM transactions WHERE txid = ?1"
        )?;

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

    pub fn apply_block(&mut self, header: BlockHeader, transactions: &[Transaction]) -> Result<()> {
        let tx = self.conn.transaction()?;

        // Calculate block hash and height
        let block_hash = {
            use crate::security_utils::sha256;
            use crate::utils::format_date;
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&header.prev_block_hash);
            bytes.extend_from_slice(&header.merkle_root);
            bytes.extend_from_slice(&header.nonce.to_be_bytes());
            bytes.extend_from_slice(format_date(&header.timestamp).as_bytes());
            sha256(&bytes)
        };

        // Get height from previous block or set to 0 for genesis
        let height: i64 = if header.prev_block_hash == [0u8; 32] {
            0
        } else {
            let mut stmt = tx.prepare(
                "SELECT height FROM block_headers WHERE block_hash = ?1"
            )?;
            let mut rows = stmt.query([header.prev_block_hash.as_slice()])?;
            if let Some(row) = rows.next()? {
                let prev_height: i64 = row.get(0)?;
                prev_height + 1
            } else {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
        };

        // Insert block header
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

        // Process each transaction
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

            // Add new UTXOs (outputs)
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
            }
        }

        tx.commit()?;
        Ok(())
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Transaction;

    #[test]
    fn test_db_creation_and_schema() {
        let db = Db::open(":memory:").unwrap();
        db.init_schema().unwrap();
        
        // Verify tables exist by trying to query them
        let mut stmt = db.conn.prepare("SELECT name FROM sqlite_master WHERE type='table'").unwrap();
        let tables: Vec<String> = stmt.query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        assert!(tables.contains(&"block_headers".to_string()));
        assert!(tables.contains(&"transactions".to_string()));
        assert!(tables.contains(&"utxos".to_string()));
    }

    #[test]
    fn test_mempool_operations() {
        let db = Db::open(":memory:").unwrap();
        db.init_schema().unwrap();

        // Create a simple transaction
        let tx = Transaction::new(vec![], vec![TxOutput {
            value: 100.0,
            address: "test_address".to_string(),
        }], Some("test".to_string()));

        // Insert into mempool
        db.insert_mempool_tx(&tx).unwrap();

        // Retrieve it
        let txid = tx.id();
        let retrieved = db.get_transaction(&txid).unwrap();
        assert!(retrieved.is_some());

        // Remove from mempool
        db.remove_mempool_tx(&txid).unwrap();
        let retrieved = db.get_transaction(&txid).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_get_utxos_for_address() {
        let db = Db::open(":memory:").unwrap();
        db.init_schema().unwrap();

        let addr = "test_address";
        let txid = [1u8; 32];

        // Manually insert a UTXO
        db.conn.execute(
            "INSERT INTO utxos (txid, vout, value, addr) VALUES (?1, ?2, ?3, ?4)",
            params![txid.as_slice(), 0i64, 50i64, addr],
        ).unwrap();

        let utxos = db.get_utxos_for_address(addr).unwrap();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].index, 0);
        assert_eq!(utxos[0].output.value, 50.0);
    }
}
