use super::Block;
use crate::{
    db::repository::LedgerRepository,
    globals::{CONFIG, CONSENSUS_RULES},
    security_utils::bytes_to_hex_string,
    utils,
};
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

    /// Calculate the expected difficulty for the next block using LWMA (zawy12).
    ///
    /// Uses a sliding window of the last `lwma_n` blocks. Each solve time is
    /// weighted linearly (oldest = weight 1, newest = weight k), so recent blocks
    /// have more influence on the result. The result is clamped to [0.5×, 2×] of
    /// the previous block's difficulty.
    pub fn calculate_next_difficulty(&self) -> usize {
        let height = self.chain.len();
        let n = CONSENSUS_RULES.lwma_n;
        let target_secs = CONSENSUS_RULES.target_block_time_secs as i64;

        if height == 0 {
            return CONSENSUS_RULES.difficulty;
        }

        let k = n.min(height);
        let window_start = height - k;

        let mut t = 0;
        let mut sum_d = 0f64;
        let mut weight = 1i64;

        for i in window_start..height {
            let prev_ts = if i > 0 {
                self.chain[i - 1].header.timestamp
            } else {
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
            sum_d += curr.header.difficulty as f64;
            weight += 1;
        }

        if t == 0 {
            return CONSENSUS_RULES.difficulty;
        }

        let avg_d = sum_d / k as f64;
        let n_sums = (k * (k + 1) / 2) as f64;
        let next_d_f = avg_d * target_secs as f64 * n_sums / t as f64;

        let prev_d = self.chain[height - 1].header.difficulty as f64;
        let next_d_clamped = next_d_f.max(prev_d * 0.5).min(prev_d * 2.0).max(1.0);

        next_d_clamped.round() as usize
    }

    /** Validate the recently mined block and if valid, add it to the chain */
    pub fn add_block(&mut self, block: Block) -> Result<(), String> {
        let last_block_hash = self.get_last_block_hash();

        if block.header.prev_block_hash != last_block_hash {
            return Err("Previous block hash does not match".to_string());
        }

        let expected_difficulty = self.calculate_next_difficulty();
        if block.header.difficulty != expected_difficulty {
            return Err(format!(
                "Invalid difficulty: expected {}, got {}",
                expected_difficulty, block.header.difficulty
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
