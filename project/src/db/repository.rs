use crate::{
    db::db,
    model::{Block, Transaction, TxOutput, UTXO, block::BlockHeader, transaction::TxId},
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
        stmt.query_map([addr], |row| build_utxo_from_row(row))?
            .collect()
    }

    pub fn get_utxos_for_addresses(&self, addrs: &[String]) -> Result<Vec<UTXO>> {
        if addrs.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = (0..addrs.len()).map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT txid, vout, value, addr FROM utxos WHERE addr IN ({})",
            placeholders.join(", ")
        );

        let mut stmt = self.conn.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            addrs.iter().map(|a| a as &dyn rusqlite::ToSql).collect();
        stmt.query_map(rusqlite::params_from_iter(params), |row| {
            build_utxo_from_row(row)
        })?
        .collect()
    }

    pub fn get_utxos_from_ids(&self, ids: &[(TxId, usize)]) -> Result<Vec<UTXO>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let conditions: Vec<String> = (0..ids.len())
            .map(|i| format!("(txid = ?{} AND vout = ?{})", i * 2 + 1, i * 2 + 2))
            .collect();
        let query = format!(
            "SELECT txid, vout, value, addr FROM utxos WHERE {}",
            conditions.join(" OR ")
        );

        let mut stmt = self.conn.prepare(&query)?;
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for (txid, vout) in ids {
            params.push(Box::new(txid.to_vec()));
            params.push(Box::new(*vout as i64));
        }
        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        stmt.query_map(rusqlite::params_from_iter(params_refs), |row| {
            build_utxo_from_row(row)
        })?
        .collect()
    }

    pub fn get_all_utxos(&self, limit: Option<usize>) -> Result<Vec<UTXO>> {
        let query = format!(
            "SELECT txid, vout, value, addr FROM utxos LIMIT {}",
            limit.unwrap_or(20)
        );
        let mut stmt = self.conn.prepare(&query)?;
        stmt.query_map([], |row| build_utxo_from_row(row))?
            .collect()
    }

    pub fn get_transaction(&self, txid: &[u8; 32]) -> Result<Option<Transaction>> {
        let mut stmt = self
            .conn
            .prepare("SELECT raw FROM transactions WHERE txid = ?1")?;
        let mut rows = stmt.query([txid.as_slice()])?;

        match rows.next()? {
            Some(row) => {
                let raw: Vec<u8> = row.get(0)?;
                let tx: Transaction = serde_json::from_slice(&raw)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok(Some(tx))
            }
            None => Ok(None),
        }
    }

    pub fn apply_block(&mut self, block: Block) -> Result<()> {
        let db_tx = self.conn.transaction()?;
        let block_hash = block.header_hash();
        let height = resolve_block_height(&db_tx, &block.header)?;

        insert_block_header(&db_tx, &block_hash, &block.header, height)?;

        for transaction in &block.transactions {
            apply_transaction(&db_tx, transaction, &block_hash, height)?;
        }

        db_tx.commit()?;
        Ok(())
    }

    pub fn get_utxo(&self, txid: TxId, vout: usize) -> Result<UTXO> {
        let mut stmt = self
            .conn
            .prepare("SELECT txid, vout, value, addr FROM utxos WHERE txid = ?1 AND vout = ?2")?;
        let mut rows = stmt.query(params![txid.as_slice(), vout as i64])?;

        match rows.next()? {
            Some(row) => build_utxo_from_row(row),
            None => Err(rusqlite::Error::QueryReturnedNoRows),
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

    pub fn get_transactions_for_address(&self, addr: &str) -> Result<Vec<[u8; 32]>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT txid FROM tx_addresses WHERE addr = ?1")?;
        stmt.query_map([addr], |row| {
            let txid_vec: Vec<u8> = row.get(0)?;
            let mut txid = [0u8; 32];
            txid.copy_from_slice(&txid_vec);
            Ok(txid)
        })?
        .collect()
    }

    pub fn has_address_been_used(&self, addr: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM tx_addresses WHERE addr = ?1 LIMIT 1")?;
        stmt.exists([addr])
    }

    pub fn has_any_address_been_used(&self, addrs: &[String]) -> Result<bool> {
        let placeholders: Vec<String> = (0..addrs.len()).map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT 1 FROM tx_addresses WHERE addr IN ({}) LIMIT 1",
            placeholders.join(", ")
        );
        let mut stmt = self.conn.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            addrs.iter().map(|a| a as &dyn rusqlite::ToSql).collect();
        stmt.exists(rusqlite::params_from_iter(params))
    }

    pub fn get_used_addresses(&self, addrs: &[String]) -> Result<Vec<(TxId, String)>> {
        let placeholders: Vec<String> = (0..addrs.len()).map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT txid, addr FROM tx_addresses WHERE addr IN ({})",
            placeholders.join(", ")
        );
        let mut stmt = self.conn.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            addrs.iter().map(|a| a as &dyn rusqlite::ToSql).collect();
        stmt.query_map(rusqlite::params_from_iter(params), |row| {
            let txid_vec: Vec<u8> = row.get(0)?;
            let mut txid = [0u8; 32];
            txid.copy_from_slice(&txid_vec);
            let addr: String = row.get(1)?;
            Ok((txid, addr))
        })?
        .collect()
    }

    pub fn get_addresses_in_transaction(&self, txid: &[u8; 32]) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT addr FROM tx_addresses WHERE txid = ?1")?;
        stmt.query_map([txid.as_slice()], |row| row.get(0))?
            .collect()
    }

    /// Rollback a block - reverses all changes made by apply_block
    pub fn rollback_block(&mut self, block: &Block) -> Result<()> {
        let block_hash = block.id();

        // Pre-fetch previous transactions before starting SQL transaction
        let mut prev_transactions = std::collections::HashMap::new();
        for transaction in &block.transactions {
            for input in &transaction.inputs {
                if !prev_transactions.contains_key(&input.prev_tx_id) {
                    let prev_tx = self
                        .get_transaction(&input.prev_tx_id)?
                        .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                    prev_transactions.insert(input.prev_tx_id, prev_tx);
                }
            }
        }

        let db_tx = self.conn.transaction()?;

        verify_is_last_block(&db_tx, &block_hash)?;

        for transaction in block.transactions.iter().rev() {
            rollback_transaction(&db_tx, transaction, &prev_transactions)?;
        }

        db_tx.execute(
            "DELETE FROM block_headers WHERE block_hash = ?1",
            [block_hash.as_slice()],
        )?;

        db_tx.commit()?;
        Ok(())
    }
}

// --- Private helper functions ---

fn resolve_block_height(db_tx: &rusqlite::Transaction, header: &BlockHeader) -> Result<i64> {
    if header.prev_block_hash == [0u8; 32] {
        return Ok(0);
    }
    let mut stmt = db_tx.prepare("SELECT height FROM block_headers WHERE block_hash = ?1")?;
    let mut rows = stmt.query([header.prev_block_hash.as_slice()])?;
    match rows.next()? {
        Some(row) => Ok(row.get::<_, i64>(0)? + 1),
        None => Err(rusqlite::Error::QueryReturnedNoRows),
    }
}

fn insert_block_header(
    db_tx: &rusqlite::Transaction,
    block_hash: &[u8; 32],
    header: &BlockHeader,
    height: i64,
) -> Result<()> {
    let timestamp = header.timestamp.and_utc().timestamp();
    db_tx.execute(
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
    Ok(())
}

fn apply_transaction(
    db_tx: &rusqlite::Transaction,
    transaction: &Transaction,
    block_hash: &[u8; 32],
    height: i64,
) -> Result<()> {
    let txid = transaction.id();
    let raw = serde_json::to_vec(transaction)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    let tx_timestamp = transaction.date.and_utc().timestamp();

    db_tx.execute(
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

    for input in &transaction.inputs {
        db_tx.execute(
            "DELETE FROM utxos WHERE txid = ?1 AND vout = ?2",
            params![input.prev_tx_id.as_slice(), input.output_index as i64],
        )?;
    }

    for (vout, output) in transaction.outputs.iter().enumerate() {
        db_tx.execute(
            "INSERT INTO utxos (txid, vout, value, addr, script)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                txid.as_slice(),
                vout as i64,
                output.value,
                &output.address,
                Vec::<u8>::new()
            ],
        )?;
        db_tx.execute(
            "INSERT OR IGNORE INTO tx_addresses (txid, addr) VALUES (?1, ?2)",
            params![txid.as_slice(), &output.address],
        )?;
    }

    Ok(())
}

fn verify_is_last_block(db_tx: &rusqlite::Transaction, block_hash: &[u8; 32]) -> Result<()> {
    let mut stmt = db_tx.prepare("SELECT COUNT(*) FROM block_headers WHERE prev_hash = ?1")?;
    let count: i64 = stmt.query_row([block_hash.as_slice()], |row| row.get(0))?;
    if count > 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

fn rollback_transaction(
    db_tx: &rusqlite::Transaction,
    transaction: &Transaction,
    prev_transactions: &std::collections::HashMap<TxId, Transaction>,
) -> Result<()> {
    let txid = transaction.id();

    // Restore spent UTXOs
    for input in &transaction.inputs {
        let prev_tx = prev_transactions
            .get(&input.prev_tx_id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let output = &prev_tx.outputs[input.output_index];

        db_tx.execute(
            "INSERT INTO utxos (txid, vout, value, addr)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                input.prev_tx_id.as_slice(),
                input.output_index as i64,
                output.value,
                &output.address
            ],
        )?;
    }

    // Remove created UTXOs
    for (vout, _) in transaction.outputs.iter().enumerate() {
        db_tx.execute(
            "DELETE FROM utxos WHERE txid = ?1 AND vout = ?2",
            params![txid.as_slice(), vout as i64],
        )?;
    }

    // Remove address tracking and transaction record
    db_tx.execute(
        "DELETE FROM tx_addresses WHERE txid = ?1",
        [txid.as_slice()],
    )?;
    db_tx.execute(
        "DELETE FROM transactions WHERE txid = ?1",
        [txid.as_slice()],
    )?;

    Ok(())
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
        output: TxOutput { value, address },
    })
}
