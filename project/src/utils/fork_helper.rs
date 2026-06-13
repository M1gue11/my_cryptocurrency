use chrono::NaiveDateTime;

use super::logger::{LogCategory, log_info, log_warning};
use crate::{
    model::{Block, block::BlockID, node::Node},
    security_utils::bytes_to_hex_string,
    utils::get_current_timestamp,
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
            timestamp: get_current_timestamp(),
        }
    }

    pub fn get_fork_start(&self) -> Option<&BlockID> {
        self.blocks_sequence.first()
    }

    pub fn append_block(&mut self, block_hash: BlockID) {
        if !self.blocks_sequence.contains(&block_hash) {
            self.blocks_sequence.push(block_hash);
        }
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

    pub fn create_or_update_fork(&mut self, _last_block: &Block, new_block: &Block) {
        for fork in &mut self.forks {
            if fork.is_block_in_branch(&new_block.header.prev_block_hash) {
                fork.append_block(new_block.id());
                return;
            }
        }
        // Always create a meaningful fork: [branching_point, new_block]
        let block_sequence = vec![new_block.header.prev_block_hash, new_block.id()];
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

    /// Register a fork starting from a known block hash (e.g., a common ancestor).
    /// Used when we know a fork exists but don't yet have the fork's blocks.
    pub fn register_fork_start(&mut self, block_hash: BlockID) {
        if self
            .forks
            .iter()
            .any(|f| f.get_fork_start() == Some(&block_hash))
        {
            return;
        }
        self.forks.push(Fork::new(vec![block_hash]));
    }

    /// Finds the longest fork that is strictly longer than the main chain.
    /// Returns the best (longest) qualifying fork for potential rebase.
    pub fn evaluate_forks(&self, node: &Node) -> Option<Fork> {
        let mut best_fork: Option<Fork> = None;
        let mut best_fork_size: usize = node.blockchain.height();

        for fork in &self.forks {
            log_info(LogCategory::Core, &format!("Evaluating fork: {:?}", fork));
            let fork_start = match fork.get_fork_start() {
                Some(hash) => hash,
                None => continue,
            };
            let fork_size = if *fork_start == [0; 32] {
                // [0; 32] is the virtual root used by the protocol to request
                // blocks from genesis. It is not stored as a real block, so a
                // root fork's chain size is the sequence without the sentinel.
                fork.blocks_sequence.len().saturating_sub(1)
            } else {
                let forked_block_height =
                    match node.blockchain.find_block_height_by_hash(*fork_start) {
                        Some(height) => height,
                        None => {
                            log_warning(
                                LogCategory::Core,
                                &format!(
                                    "Could not find forked block height for hash: {}",
                                    bytes_to_hex_string(fork_start)
                                ),
                            );
                            continue;
                        }
                    };
                log_info(
                    LogCategory::Core,
                    &format!("Forked block height: {}", forked_block_height),
                );
                fork.blocks_sequence.len() + forked_block_height
            };
            log_info(
                LogCategory::Core,
                &format!(
                    "Calculated fork size: {} - BC height: {}",
                    fork_size,
                    node.blockchain.height()
                ),
            );
            if fork_size > best_fork_size {
                log_info(
                    LogCategory::Core,
                    &format!(
                        "Found a fork with size {} that is larger than current best {}",
                        fork_size, best_fork_size
                    ),
                );
                best_fork = Some(fork.clone());
                best_fork_size = fork_size;
            }
        }
        best_fork
    }

    pub fn clear_forks(&mut self) {
        self.forks.clear();
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use primitive_types::U256;

    use super::{Fork, ForkHelper};
    use crate::{
        db::db::init_db,
        model::{Block, block::BlockHeader, node::Node},
    };

    fn test_block(prev_block_hash: [u8; 32], nonce: u32) -> Block {
        let timestamp = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, nonce)
            .unwrap();

        Block {
            header: BlockHeader {
                prev_block_hash,
                merkle_root: [nonce as u8; 32],
                nonce,
                timestamp,
                target: U256::MAX,
            },
            transactions: Vec::new(),
        }
    }

    fn test_node_with_chain(chain: Vec<Block>) -> Node {
        init_db();
        let mut node = Node::new();
        node.blockchain.chain = chain;
        node
    }

    #[test]
    fn evaluates_root_fork_using_virtual_root_height() {
        let local_genesis = test_block([0; 32], 1);
        let node = test_node_with_chain(vec![local_genesis]);

        let fork_genesis = test_block([0; 32], 2);
        let fork_second = test_block(fork_genesis.id(), 3);
        let helper = ForkHelper {
            forks: vec![Fork::new(vec![
                [0; 32],
                fork_genesis.id(),
                fork_second.id(),
            ])],
        };

        let best_fork = helper.evaluate_forks(&node).expect("root fork should win");

        assert_eq!(best_fork.get_fork_start(), Some(&[0; 32]));
    }

    #[test]
    fn ignores_root_fork_when_not_longer_than_local_chain() {
        let local_genesis = test_block([0; 32], 1);
        let local_second = test_block(local_genesis.id(), 2);
        let node = test_node_with_chain(vec![local_genesis, local_second]);

        let fork_genesis = test_block([0; 32], 3);
        let helper = ForkHelper {
            forks: vec![Fork::new(vec![[0; 32], fork_genesis.id()])],
        };

        assert!(helper.evaluate_forks(&node).is_none());
    }
}
