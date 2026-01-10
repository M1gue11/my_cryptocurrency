use std::collections::HashSet;

use super::Transaction;
use crate::globals::CONSENSUS_RULES;
use crate::security_utils::hash_starts_with_zero_bits;
use crate::{
    security_utils::{bytes_to_hex_string, sha256},
    utils::{MerkleTree, format_date},
};
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

    pub fn evaluate_merkle_root(&mut self) {
        let leaf_hashes = self.transactions.iter().map(|tx| tx.id()).collect();
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

    pub fn size(&self) -> usize {
        let mut size = 0;
        size += self.header.prev_block_hash.len();
        size += self.header.merkle_root.len();
        size += std::mem::size_of_val(&self.header.nonce);
        size += std::mem::size_of_val(&self.header.timestamp);
        for tx in &self.transactions {
            size += tx.as_bytes().len();
        }
        size
    }

    pub fn header_hash(&self) -> [u8; 32] {
        sha256(&self.header_bytes())
    }

    pub fn id(&self) -> [u8; 32] {
        self.header_hash()
    }

    pub fn eval_merkle_root_from_transactions(txs: &[Transaction]) -> [u8; 32] {
        let mut leaf_hashes: Vec<[u8; 32]> = Vec::new();
        for tx in txs {
            leaf_hashes.push(tx.id());
        }
        let merkle_tree = MerkleTree::from_leaves(leaf_hashes);
        merkle_tree.root()
    }

    /** Staticaly validate the block without external dependencies
     * Checks:
     * - Block has at least one transaction
     * - Proof of work is valid
     * - Merkle root is valid
     * - All transactions are valid
     * - No double spending within the block
     */
    pub fn validate(&self) -> Result<(), String> {
        if self.transactions.is_empty() {
            return Err("Block has no transactions".to_string());
        }

        if self.size() > (CONSENSUS_RULES.max_block_size_kb * 1000.0) as usize {
            return Err(format!(
                "Block size exceeds maximum limit: {} bytes",
                self.size()
            ));
        }

        if !hash_starts_with_zero_bits(&self.header_hash(), CONSENSUS_RULES.difficulty) {
            return Err("Invalid proof of work".to_string());
        }

        if Block::eval_merkle_root_from_transactions(&self.transactions) != self.header.merkle_root
        {
            return Err("Invalid Merkle root".to_string());
        }

        let mut unique_utxos_map = HashSet::new();
        for tx in &self.transactions {
            if let Err(e) = tx.validate() {
                return Err(e.to_string());
            }
            for input in &tx.inputs {
                if unique_utxos_map.contains(&input.prev_tx_id) {
                    return Err(format!(
                        "Double spending detected in block for UTXO: tx_id: {}, output_index: {}",
                        bytes_to_hex_string(&input.prev_tx_id),
                        input.output_index
                    ));
                }

                unique_utxos_map.insert(input.prev_tx_id);
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Block")
            .field(
                "prev_block_hash",
                &bytes_to_hex_string(&self.header.prev_block_hash),
            )
            .field(
                "merkle root",
                &bytes_to_hex_string(&self.header.merkle_root),
            )
            .field("transactions", &self.transactions)
            .field("nonce", &self.header.nonce)
            .field("date", &self.header.timestamp)
            .finish()
    }
}
