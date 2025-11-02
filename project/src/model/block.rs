use crate::{
    security_utils::{digest_to_hex_string, sha256},
    utils::{MerkleTree, format_date},
};

use super::Transaction;
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader {
    pub prev_block_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub nonce: u32,
    pub timestamp: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(prev_block_hash: [u8; 32]) -> Self {
        let timestamp = Utc::now().naive_utc();
        let header = BlockHeader {
            prev_block_hash,
            merkle_root: [0; 32],
            nonce: 0,
            timestamp,
        };
        Block {
            header,
            transactions: Vec::new(),
        }
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }

    pub fn evaluate_merkle_root(&mut self) {
        let mut leaf_hashes: Vec<[u8; 32]> = Vec::new();
        for tx in &self.transactions {
            leaf_hashes.push(tx.id());
        }
        let merkle_tree = MerkleTree::from_leaves(leaf_hashes);
        self.header.merkle_root = merkle_tree.root();
    }

    pub fn header_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.header.prev_block_hash);
        out.extend_from_slice(&self.header.merkle_root);
        out.extend_from_slice(&self.header.nonce.to_be_bytes());
        out.extend_from_slice(format_date(&self.header.timestamp).as_bytes());
        out
    }

    pub fn header_hash(&self) -> [u8; 32] {
        sha256(&self.header_bytes())
    }
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Block")
            .field(
                "prev_block_hash",
                &digest_to_hex_string(&self.header.prev_block_hash),
            )
            .field(
                "merkle root",
                &digest_to_hex_string(&self.header.merkle_root),
            )
            .field("transactions", &self.transactions)
            .field("nonce", &self.header.nonce)
            .field("date", &self.header.timestamp)
            .finish()
    }
}
