use crate::{
    db::db,
    model::{Block, Transaction, TxOutput, UTXO, transaction::TxId},
};
use rusqlite::{Result, params};

pub struct LedgerRepository {
    conn: db::DbConnection,
}

impl LedgerRepository {
    pub fn new() -> Self {
        let conn = db::get_db().get_conn();
        LedgerRepository { conn }
    }

    pub fn get_utxos_for_address(&self, addr: &str) -> Result<Vec<UTXO>> {
        let mut stmt = self
            .conn
            .prepare("SELECT txid, vout, value, addr FROM utxos WHERE addr = ?1")?;

        let utxos = stmt.query_map([addr], |row| build_utxo_from_row(row))?;

        let mut result = Vec::new();
        for utxo in utxos {
            result.push(utxo?);
        }
        Ok(result)
    }

    pub fn get_utxos_for_addresses(&self, addrs: &[String]) -> Result<Vec<UTXO>> {
        if addrs.is_empty() {
            return Ok(Vec::new());
        }

        let mut query = String::from("SELECT txid, vout, value, addr FROM utxos WHERE addr IN (");
        let placeholders: Vec<String> = (0..addrs.len()).map(|_| "?".to_string()).collect();
        query.push_str(&placeholders.join(", "));
        query.push(')');

        let mut stmt = self.conn.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            addrs.iter().map(|a| a as &dyn rusqlite::ToSql).collect();

        let utxos = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            build_utxo_from_row(row)
        })?;

        let mut result = Vec::new();
        for utxo in utxos {
            result.push(utxo?);
        }
        Ok(result)
    }

    pub fn get_utxos_from_ids(&self, ids: &[(TxId, usize)]) -> Result<Vec<UTXO>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut query = String::from("SELECT txid, vout, value, addr FROM utxos WHERE ");
        let conditions: Vec<String> = (0..ids.len())
            .map(|i| format!("(txid = ?{} AND vout = ?{})", i * 2 + 1, i * 2 + 2))
            .collect();
        query.push_str(&conditions.join(" OR "));

        let mut stmt = self.conn.prepare(&query)?;

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for (txid, vout) in ids {
            params.push(Box::new(txid.to_vec()));
            params.push(Box::new(*vout as i64));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let utxos = stmt.query_map(rusqlite::params_from_iter(params_refs), |row| {
            build_utxo_from_row(row)
        })?;

        let mut result = Vec::new();
        for utxo in utxos {
            result.push(utxo?);
        }
        Ok(result)
    }

    pub fn get_all_utxos(&self, limit: Option<usize>) -> Result<Vec<UTXO>> {
        let query = format!(
            "SELECT txid, vout, value, addr FROM utxos LIMIT {}",
            limit.unwrap_or_else(|| 20)
        );
        let mut stmt = self.conn.prepare(&query)?;
        let utxos = stmt.query_map([], |row| build_utxo_from_row(row))?;
        let mut result = Vec::new();
        for utxo in utxos {
            result.push(utxo?);
        }
        Ok(result)
    }

    pub fn get_transaction(&self, txid: &[u8; 32]) -> Result<Transaction> {
        let mut stmt = self
            .conn
            .prepare("SELECT raw FROM transactions WHERE txid = ?1")?;

        let mut rows = stmt.query([txid.as_slice()])?;

        if let Some(row) = rows.next()? {
            let raw: Vec<u8> = row.get(0)?;
            let tx: Transaction = serde_json::from_slice(&raw)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(tx)
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
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

    pub fn get_utxo(&self, txid: TxId, vout: usize) -> Result<UTXO> {
        let mut stmt = self
            .conn
            .prepare("SELECT txid, vout, value, addr FROM utxos WHERE txid = ?1 AND vout = ?2")?;

        let mut rows = stmt.query(params![txid.as_slice(), vout as i64])?;

        if let Some(row) = rows.next()? {
            build_utxo_from_row(&row)
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
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

    pub fn get_used_addresses(&self, addrs: &[String]) -> Result<Vec<(TxId, String)>> {
        let mut query = String::from("SELECT txid, addr FROM tx_addresses WHERE addr IN (");
        let placeholders: Vec<String> = (0..addrs.len()).map(|_| "?".to_string()).collect();
        query.push_str(&placeholders.join(", "));
        query.push_str(")");

        let mut stmt = self.conn.prepare(&query)?;

        let params: Vec<&dyn rusqlite::ToSql> =
            addrs.iter().map(|a| a as &dyn rusqlite::ToSql).collect();

        let used_addresses = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let txid_vec: Vec<u8> = row.get(0)?;
            let mut txid = [0u8; 32];
            txid.copy_from_slice(&txid_vec);
            let addr: String = row.get(1)?;
            Ok((txid, addr))
        })?;
        let mut result = Vec::new();
        for ua in used_addresses {
            result.push(ua?);
        }
        Ok(result)
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

fn build_utxo_from_row(row: &rusqlite::Row) -> rusqlite::Result<UTXO> {
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
}
