use super::Block;
use crate::config::CONFIG;
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
        match Self::load_chain(None) {
            Ok(blockchain) => blockchain,
            Err(_) => {
                let mut chain = Vec::new();
                let genesis_block = Block::new("0".into());
                chain.push(genesis_block);
                Blockchain { chain }
            }
        }
    }

    pub fn get_last_block_hash(&self) -> String {
        self.chain
            .last()
            .expect("Blockchain is empty")
            .calculate_hash()
    }

    pub fn add_block(&mut self, block: Block, difficulty: usize) -> bool {
        let last_block = self
            .chain
            .last()
            .expect("The chain should have at least one block.");
        let prefix = "0".repeat(difficulty);

        if block.prev_block_hash != last_block.calculate_hash() {
            println!("ERROR: Previous block hash does not match!");
            return false;
        }

        if !block.calculate_hash().starts_with(&prefix) {
            println!("ERROR: Invalid proof of work!");
            return false;
        }
        self.chain.push(block);
        return true;
    }

    pub fn persist_chain(&self, path: Option<String>) {
        let path = path.unwrap_or_else(|| CONFIG.persisted_chain_path.to_string());
        let dir_path = std::path::Path::new(&path);
        if !dir_path.exists() {
            std::fs::create_dir_all(&dir_path)
                .expect("Failed to create directory for blockchain file");
        }

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
            eprintln!("Failed to load blockchain: {}", e);
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })
    }
}
