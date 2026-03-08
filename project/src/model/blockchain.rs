use super::Block;
use crate::{
    db::repository::LedgerRepository,
    globals::{CONFIG, CONSENSUS_RULES},
    security_utils::bytes_to_hex_string,
    utils,
};
use primitive_types::U256;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, BufWriter},
};

const BLOCKCHAIN_FILE: &str = "bc.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct Blockchain {
    pub chain: Vec<Block>,
}

impl Blockchain {
    pub fn new() -> Self {
        Blockchain { chain: Vec::new() }
    }

    pub fn get_last_block(&self) -> Option<&Block> {
        self.chain.last()
    }

    pub fn get_last_block_hash(&self) -> [u8; 32] {
        match self.get_last_block() {
            Some(block) => block.header_hash(),
            None => [0; 32],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }

    /// Calculate the target for the next block using LWMA (zawy12).
    pub fn calculate_next_target(&self) -> U256 {
        let height = self.chain.len();
        let lwma_n = CONSENSUS_RULES.lwma_n;
        let target_secs = CONSENSUS_RULES.target_block_time_secs as i64;

        if height == 0 {
            return CONSENSUS_RULES.initial_target;
        }

        let k = lwma_n.min(height);
        let window_start = height - k;

        let mut t: i64 = 0;
        let mut sum_target = U256::zero();
        let mut weight = 1i64;

        for i in window_start..height {
            let prev_ts = if i > 0 {
                // normal case: use timestamp of previous block
                self.chain[i - 1].header.timestamp
            } else {
                // edge case: if we're at the first block, use its timestamp as "previous"
                self.chain[0].header.timestamp
            };
            let curr = &self.chain[i];

            let solvetime = curr
                .header
                .timestamp
                .signed_duration_since(prev_ts)
                .num_seconds()
                .max(1)
                .min(6 * target_secs);

            t += solvetime * weight;
            sum_target = sum_target + curr.header.target;
            weight += 1;
        }

        if t == 0 {
            return CONSENSUS_RULES.initial_target;
        }

        let avg_target = sum_target / U256::from(k);
        let n_sums = U256::from((k * (k + 1) / 2) as u64);
        let denom = U256::from(target_secs as u64) * n_sums;
        let t_u256 = U256::from(t as u64);

        // next_target = avg_target * t / (target_secs * n_sums)
        // When t > denom: blocks were slow -> target increases (easier)
        // When t < denom: blocks were fast -> target decreases (harder)
        // Use checked_mul to guard against overflow; saturate to U256::MAX if needed.
        let next_target = match avg_target.checked_mul(t_u256) {
            Some(product) => product / denom,
            None => {
                // Overflow: blocks extremely slow, use maximum possible target
                U256::MAX
            }
        };

        let prev_target = self.chain[height - 1].header.target;
        next_target
            .max(prev_target / 2)
            .min(match prev_target.checked_mul(U256::from(2u32)) {
                Some(v) => v,
                None => U256::MAX,
            })
            .max(U256::one())
    }

    /** Validate the recently mined block and if valid, add it to the chain */
    pub fn add_block(&mut self, block: Block) -> Result<(), String> {
        let last_block_hash = self.get_last_block_hash();

        if block.header.prev_block_hash != last_block_hash {
            return Err("Previous block hash does not match".to_string());
        }

        let expected_target = self.calculate_next_target();
        if block.header.target != expected_target {
            return Err(format!(
                "Invalid target: expected {:x}, got {:x}",
                expected_target, block.header.target
            ));
        }

        if let Err(e) = block.validate() {
            return Err(format!("Block validation failed: {}", e));
        }
        let repo = LedgerRepository::new();
        for tx in &block.transactions {
            for input in &tx.inputs {
                let input_utxo = repo.get_utxo(input.prev_tx_id, input.output_index);
                if input_utxo.is_err() {
                    return Err(format!(
                        "Transaction input is not a valid UTXO: tx_id: {}, output_index: {}",
                        bytes_to_hex_string(&input.prev_tx_id),
                        input.output_index
                    ));
                }
            }
        }
        self.chain.push(block);
        Ok(())
    }

    pub fn find_block_by_hash(&self, hash: [u8; 32]) -> Option<&Block> {
        self.chain.iter().find(|block| block.header_hash() == hash)
    }

    pub fn find_block_height_by_hash(&self, hash: [u8; 32]) -> Option<usize> {
        self.chain
            .iter()
            .position(|block| block.header_hash() == hash)
    }

    pub fn build_block_sequence(&self) -> Vec<[u8; 32]> {
        self.chain.iter().map(|block| block.header_hash()).collect()
    }

    pub fn height(&self) -> usize {
        self.chain.len()
    }

    pub fn persist_chain(&self, path: Option<String>) {
        let path = path.unwrap_or_else(|| CONFIG.persisted_chain_path.to_string());
        utils::assert_parent_dir_exists(&path)
            .expect("Failed to create parent directories for blockchain file");

        let file = File::create(format!("{}/{}", path, BLOCKCHAIN_FILE))
            .expect("Failed to create blockchain file");
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &self).expect("Failed to write blockchain to file");
    }

    pub fn load_chain(path: Option<String>) -> Result<Self, std::io::Error> {
        let path = path.unwrap_or_else(|| CONFIG.persisted_chain_path.to_string());
        let file_path = format!("{}/{}", path, BLOCKCHAIN_FILE);

        let file = File::open(&file_path)?;
        let rdr = BufReader::new(file);

        serde_json::from_reader(rdr).map_err(|e| {
            utils::log_error(
                utils::LogCategory::Core,
                &format!("Failed to load blockchain: {}", e),
            );
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })
    }
}
