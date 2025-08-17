use crate::security_utils::{digest_to_hex_string, sha256};

use super::Transaction;
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Block {
    pub prev_block_hash: [u8; 32],
    pub transactions: Vec<Transaction>,
    pub nonce: u32,
    pub date: NaiveDateTime,
}

impl Block {
    pub fn new(prev_block_hash: [u8; 32]) -> Self {
        let date = Utc::now().naive_utc();
        Block {
            nonce: 0,
            transactions: Vec::new(),
            prev_block_hash,
            date,
        }
    }

    pub fn calculate_hash(&self) -> [u8; 32] {
        let data = format!(
            "{}{}{:?}{:?}",
            self.nonce,
            digest_to_hex_string(&self.prev_block_hash),
            self.transactions,
            self.date
        );
        sha256(data.as_bytes())
    }
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Block")
            .field(
                "prev_block_hash",
                &digest_to_hex_string(&self.prev_block_hash),
            )
            .field("transactions", &self.transactions)
            .field("nonce", &self.nonce)
            .field("date", &self.date)
            .finish()
    }
}
