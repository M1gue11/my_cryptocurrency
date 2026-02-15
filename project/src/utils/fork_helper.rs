use chrono::{NaiveDateTime, Utc};

use crate::{
    model::{Block, block::BlockID, node::Node},
    security_utils::bytes_to_hex_string,
};

#[derive(Clone)]
pub struct Fork {
    pub blocks_sequence: Vec<BlockID>,
    pub timestamp: NaiveDateTime,
}

impl std::fmt::Debug for Fork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let blocks: Vec<String> = self
            .blocks_sequence
            .iter()
            .map(|id| bytes_to_hex_string(id)[..12].to_string())
            .collect();
        write!(
            f,
            "Fork(len={}, start={}, blocks=[{}], created={})",
            self.blocks_sequence.len(),
            blocks.first().unwrap_or(&"empty".to_string()),
            blocks.join(" -> "),
            self.timestamp.format("%H:%M:%S")
        )
    }
}

impl Fork {
    pub fn new(blocks_sequence: Vec<BlockID>) -> Self {
        Self {
            blocks_sequence,
            timestamp: Utc::now().naive_utc(),
        }
    }

    pub fn get_fork_start(&self) -> Option<&BlockID> {
        self.blocks_sequence.first()
    }

    pub fn append_block(&mut self, block_hash: BlockID) {
        self.blocks_sequence.push(block_hash);
    }

    pub fn is_block_in_branch(&self, block_hash: &BlockID) -> bool {
        self.blocks_sequence.contains(block_hash)
    }
}

pub struct ForkHelper {
    pub forks: Vec<Fork>,
}

impl ForkHelper {
    pub fn new() -> Self {
        Self { forks: Vec::new() }
    }

    pub fn create_or_update_fork(&mut self, last_block: &Block, new_block: &Block) {
        for fork in &mut self.forks {
            // Check if prev block hash of the new block matches any block in the fork
            if fork.is_block_in_branch(&new_block.header.prev_block_hash) {
                fork.append_block(new_block.id());
                return;
            }
        }
        // If no existing fork matches, create a new one
        let mut block_sequence = Vec::with_capacity(2);
        if last_block.header.prev_block_hash == new_block.header.prev_block_hash {
            // This means the new block is a direct sibling of the last block, so we start the fork from their common parent
            block_sequence.push(new_block.header.prev_block_hash);
            block_sequence.push(new_block.id());
        }
        let new_fork = Fork::new(block_sequence);
        self.forks.push(new_fork);
    }

    /// Returns true if a new fork was created or updated, false if the new block simply extends the main chain
    pub fn verify_fork(&mut self, last_block: &Block, new_block: &Block) -> bool {
        if new_block.header.prev_block_hash == last_block.id() {
            return false;
        }
        self.create_or_update_fork(last_block, new_block);
        true
    }

    /// Finds the longest fork without modifying the fork list.
    ///
    /// If a fork is found that has more blocks than the main chain,
    /// it returns that fork for potential rebase
    pub fn evaluate_forks(&self, node: &Node) -> Option<Fork> {
        let mut fork_to_rebase: Option<Fork> = None;
        for fork in &self.forks {
            println!("Evaluating fork: {:?}", fork);
            let fork_start = match fork.get_fork_start() {
                Some(hash) => hash,
                None => continue, // Skip empty forks
            };
            let forked_block_height = match node.blockchain.find_block_height_by_hash(*fork_start) {
                Some(height) => height,
                None => {
                    // Skip forks with unknown starting block height
                    println!(
                        "Could not find forked block height for hash: {}",
                        bytes_to_hex_string(fork_start)
                    );
                    continue;
                }
            };
            println!("Forked block height: {}", forked_block_height);
            let fork_size = fork.blocks_sequence.len() + forked_block_height;
            println!(
                "Calculated fork size: {} - BC height: {}",
                fork_size,
                node.blockchain.height()
            );
            if fork_size > node.blockchain.height() {
                println!(
                    "Found a fork with size {} that is larger than the main chain height {}",
                    fork_size,
                    node.blockchain.height()
                );
                fork_to_rebase = Some(fork.clone());
            }
        }
        fork_to_rebase
    }

    /// Removes a fork from the list by timestamp
    pub fn remove_fork(&mut self, fork: &Fork) {
        self.forks.retain(|f| f.timestamp != fork.timestamp);
    }
}
