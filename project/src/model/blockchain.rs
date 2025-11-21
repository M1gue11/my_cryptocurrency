use super::Block;
use crate::globals::CONFIG;
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

    pub fn get_last_block_hash(&self) -> [u8; 32] {
        match self.chain.last() {
            Some(block) => block.header_hash(),
            None => [0; 32],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }

    pub fn add_block(&mut self, block: Block) -> Result<(), String> {
        let last_block_hash = self.get_last_block_hash();

        if block.header.prev_block_hash != last_block_hash {
            return Err("Previous block hash does not match".to_string());
        }

        let block_validation = block.validate();
        match block_validation {
            Ok(()) => self.chain.push(block),
            Err(e) => return Err(e),
        };
        Ok(())
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
