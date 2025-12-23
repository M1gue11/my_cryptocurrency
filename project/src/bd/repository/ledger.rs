use crate::model::transaction::TxId;
use crate::model::{Block, Transaction, TxOutput, UTXO};
use rusqlite::{Connection, Result, params};

pub struct LedgerRepository<'a> {
    read: &'a Connection,
    write: &'a Connection,
}

impl<'a> LedgerRepository<'a> {
    pub(crate) fn new(read: &'a Connection, write: &'a Connection) -> Self {
        LedgerRepository { read, write }
    }

    pub fn get_utxos_for_address(&self, addr: &str) -> Result<Vec<UTXO>> {
        let mut stmt = self
            .read
            .prepare("SELECT txid, vout, value, addr FROM utxos WHERE addr = ?1")?;

        let utxos = stmt.query_map([addr], map_utxo_row)?;

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

        let mut stmt = self.read.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            addrs.iter().map(|a| a as &dyn rusqlite::ToSql).collect();

        let utxos = stmt.query_map(rusqlite::params_from_iter(params), map_utxo_row)?;

        let mut result = Vec::new();
        for utxo in utxos {
            result.push(utxo?);
        }
        Ok(result)
    }

    pub fn get_transaction(&self, txid: &[u8; 32]) -> Result<Option<Transaction>> {
        let mut stmt = self
            .read
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

    pub fn apply_block(&self, block: Block, transactions: &[Transaction]) -> Result<()> {
        let tx = unsafe {
            let conn = self.write as *const Connection as *mut Connection;
            (*conn).transaction()?
        };

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

            for input in &transaction.inputs {
                tx.execute(
                    "DELETE FROM utxos WHERE txid = ?1 AND vout = ?2",
                    params![input.prev_tx_id.as_slice(), input.output_index as i64],
                )?;
            }

            for (vout, output) in transaction.outputs.iter().enumerate() {
                tx.execute(
                    "INSERT INTO utxos (txid, vout, value, addr, script)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        txid.as_slice(),
                        vout as i64,
                        output.value as i64,
                        &output.address,
                        Vec::<u8>::new()
                    ],
                )?;

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
            .read
            .prepare("SELECT txid, vout, value, addr FROM utxos WHERE txid = ?1 AND vout = ?2")?;

        let mut rows = stmt.query(params![txid.as_slice(), vout as i64])?;

        if let Some(row) = rows.next()? {
            map_utxo_row(row)
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }
    }

    pub fn insert_mempool_tx(&self, tx: &Transaction) -> Result<()> {
        let txid = tx.id();
        let raw = serde_json::to_vec(tx)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let timestamp = tx.date.and_utc().timestamp();

        self.write.execute(
            "INSERT OR REPLACE INTO transactions (txid, raw, block_hash, block_height, timestamp)
             VALUES (?1, ?2, NULL, NULL, ?3)",
            params![txid.as_slice(), raw, timestamp],
        )?;

        Ok(())
    }

    pub fn remove_mempool_tx(&self, txid: &[u8; 32]) -> Result<()> {
        self.write.execute(
            "DELETE FROM transactions WHERE txid = ?1 AND block_hash IS NULL",
            [txid.as_slice()],
        )?;
        Ok(())
    }

    pub fn get_transactions_for_address(&self, addr: &str) -> Result<Vec<[u8; 32]>> {
        let mut stmt = self
            .read
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

    pub fn has_address_been_used(&self, addr: &str) -> Result<bool> {
        let mut stmt = self
            .read
            .prepare("SELECT 1 FROM tx_addresses WHERE addr = ?1 LIMIT 1")?;

        let has_address = stmt.exists([addr])?;
        Ok(has_address)
    }

    pub fn has_any_address_been_used(&self, addrs: &[String]) -> Result<bool> {
        if addrs.is_empty() {
            return Ok(false);
        }

        let mut query = String::from("SELECT 1 FROM tx_addresses WHERE addr IN (");
        let placeholders: Vec<String> = (0..addrs.len()).map(|_| "?".to_string()).collect();
        query.push_str(&placeholders.join(", "));
        query.push_str(") LIMIT 1");

        let mut stmt = self.read.prepare(&query)?;

        let params: Vec<&dyn rusqlite::ToSql> =
            addrs.iter().map(|a| a as &dyn rusqlite::ToSql).collect();

        let has_address = stmt.exists(rusqlite::params_from_iter(params))?;
        Ok(has_address)
    }

    pub fn get_addresses_in_transaction(&self, txid: &[u8; 32]) -> Result<Vec<String>> {
        let mut stmt = self
            .read
            .prepare("SELECT DISTINCT addr FROM tx_addresses WHERE txid = ?1")?;

        let addresses = stmt.query_map([txid.as_slice()], |row| row.get(0))?;

        let mut result = Vec::new();
        for addr in addresses {
            result.push(addr?);
        }
        Ok(result)
    }
}

fn map_utxo_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UTXO> {
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
