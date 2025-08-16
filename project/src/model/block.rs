use super::Transaction;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    pub nonce: u64,
    pub transactions: Vec<Transaction>,
    pub prev_block_hash: String,
}

impl Block {
    pub fn new(prev_block_hash: String) -> Self {
        let id = Uuid::new_v4();
        Block {
            id,
            nonce: 0,
            transactions: Vec::new(),
            prev_block_hash,
        }
    }

    pub fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        let data = format!(
            "{:?} {} {} {:?}",
            self.id, self.nonce, self.prev_block_hash, self.transactions
        );
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.id, self.amount, self.origin_addr, self.destination_addr
        )
    }
}
