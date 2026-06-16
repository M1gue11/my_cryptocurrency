use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;

use chrono::NaiveDateTime;

use super::logger::{LogCategory, log_info, log_warning};
use crate::globals::CONFIG;
use crate::{
    model::{Block, Blockchain, block::BlockID},
    security_utils::bytes_to_hex_string,
    utils::get_current_timestamp,
};

#[derive(Debug, Clone)]
pub enum ForkUpdateStatus {
    Stored,
    DuplicateMainChain,
    DuplicateForkTree,
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct ReorgCandidate {
    pub ancestor_hash: BlockID,
    pub blocks: Vec<Block>,
    pub candidate_height: usize,
}

#[derive(Debug, Clone)]
pub struct ForkUpdate {
    pub status: ForkUpdateStatus,
    pub missing_parents: Vec<BlockID>,
    pub connectable_blocks: Vec<Block>,
    pub best_reorg: Option<ReorgCandidate>,
}

impl ForkUpdate {
    fn empty(status: ForkUpdateStatus) -> Self {
        Self {
            status,
            missing_parents: Vec::new(),
            connectable_blocks: Vec::new(),
            best_reorg: None,
        }
    }
}

#[derive(Clone)]
pub struct ForkNode {
    pub block: Block,
    pub parent: BlockID,
    pub children: HashSet<BlockID>,
    pub first_seen_at: NaiveDateTime,
    pub source_peer: Option<SocketAddr>,
}

impl std::fmt::Debug for ForkNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForkNode")
            .field("block", &bytes_to_hex_string(&self.block.id()))
            .field("parent", &bytes_to_hex_string(&self.parent))
            .field("children", &self.children.len())
            .field("first_seen_at", &self.first_seen_at)
            .field("source_peer", &self.source_peer)
            .finish()
    }
}

pub struct ForkHelper {
    nodes: HashMap<BlockID, ForkNode>,
    children_by_parent: HashMap<BlockID, HashSet<BlockID>>,
    max_blocks: usize,
}

impl ForkHelper {
    pub fn new() -> Self {
        Self::with_capacity_limit(CONFIG.max_fork_blocks)
    }

    pub fn with_capacity_limit(max_blocks: usize) -> Self {
        Self {
            nodes: HashMap::with_capacity(max_blocks),
            children_by_parent: HashMap::with_capacity(max_blocks),
            max_blocks,
        }
    }

    pub fn observe_block(
        &mut self,
        blockchain: &Blockchain,
        block: Block,
        source_peer: Option<SocketAddr>,
    ) -> ForkUpdate {
        let block_hash = block.id();

        if blockchain.find_block_by_hash(block_hash).is_some() {
            return ForkUpdate::empty(ForkUpdateStatus::DuplicateMainChain);
        }

        if self.nodes.contains_key(&block_hash) {
            return ForkUpdate::empty(ForkUpdateStatus::DuplicateForkTree);
        }

        if let Err(e) = block.validate() {
            return ForkUpdate::empty(ForkUpdateStatus::Invalid(e));
        }

        self.insert_node(block, source_peer);

        let missing_parents = self.find_missing_parents(blockchain);
        let connectable_blocks = self.take_connectable_blocks(blockchain.get_last_block_hash());
        let best_reorg = self.find_best_reorg_candidate(blockchain);

        self.prune_to_capacity();

        ForkUpdate {
            status: ForkUpdateStatus::Stored,
            missing_parents,
            connectable_blocks,
            best_reorg,
        }
    }

    pub fn contains_block(&self, block_hash: &BlockID) -> bool {
        self.nodes.contains_key(block_hash)
    }

    pub fn clear_forks(&mut self) {
        self.nodes.clear();
        self.children_by_parent.clear();
    }

    pub fn find_best_reorg_candidate(&self, blockchain: &Blockchain) -> Option<ReorgCandidate> {
        let mut best: Option<ReorgCandidate> = None;

        for leaf_hash in self.leaf_hashes() {
            let Some(candidate) = self.build_candidate_from_leaf(blockchain, leaf_hash) else {
                continue;
            };

            log_info(
                LogCategory::Core,
                &format!(
                    "Evaluating fork-tree candidate ending at {} with height {} - BC height: {}",
                    bytes_to_hex_string(&leaf_hash),
                    candidate.candidate_height,
                    blockchain.height()
                ),
            );

            if candidate.candidate_height <= blockchain.height() {
                continue;
            }

            match &best {
                Some(current_best)
                    if current_best.candidate_height > candidate.candidate_height => {}
                Some(current_best)
                    if current_best.candidate_height == candidate.candidate_height =>
                {
                    let current_leaf = current_best.blocks.last().map(|b| b.id());
                    let candidate_leaf = candidate.blocks.last().map(|b| b.id());

                    if candidate_leaf < current_leaf {
                        best = Some(candidate);
                    }
                }
                _ => best = Some(candidate),
            }
        }

        best
    }

    pub fn find_missing_parents(&self, blockchain: &Blockchain) -> Vec<BlockID> {
        let mut missing = Vec::new();
        let mut seen = HashSet::new();

        for node in self.nodes.values() {
            let parent = node.parent;

            if parent == [0; 32] {
                continue;
            }

            let parent_in_main_chain = blockchain.find_block_by_hash(parent).is_some();
            let parent_in_fork_tree = self.nodes.contains_key(&parent);

            if !parent_in_main_chain && !parent_in_fork_tree && seen.insert(parent) {
                missing.push(parent);
            }
        }

        missing
    }

    pub fn take_connectable_blocks(&mut self, mut parent_hash: BlockID) -> Vec<Block> {
        let mut blocks = Vec::new();

        loop {
            let children = self
                .children_by_parent
                .get(&parent_hash)
                .cloned()
                .unwrap_or_default();

            if children.len() != 1 {
                break;
            }

            let child_hash = *children.iter().next().unwrap();
            let Some(block) = self.remove_node_only(child_hash) else {
                break;
            };

            parent_hash = child_hash;
            blocks.push(block);
        }

        blocks
    }

    pub fn prune_subtree(&mut self, root_hash: BlockID) {
        let mut stack = vec![root_hash];

        while let Some(hash) = stack.pop() {
            let indexed_children = self.children_by_parent.remove(&hash).unwrap_or_default();

            if let Some(node) = self.nodes.remove(&hash) {
                for child in &node.children {
                    stack.push(*child);
                }
                self.detach_from_parent_index(node.parent, hash);
            }

            for child in indexed_children {
                stack.push(child);
            }
        }
    }

    pub fn remove_applied_blocks(&mut self, blocks: &[Block]) {
        let hashes: HashSet<BlockID> = blocks.iter().map(|block| block.id()).collect();

        for hash in &hashes {
            self.nodes.remove(hash);
        }

        self.rebuild_children_index();
    }

    fn insert_node(&mut self, block: Block, source_peer: Option<SocketAddr>) {
        let block_hash = block.id();
        let parent_hash = block.header.prev_block_hash;
        let existing_children = self
            .children_by_parent
            .remove(&block_hash)
            .unwrap_or_default();

        let node = ForkNode {
            block,
            parent: parent_hash,
            children: existing_children.clone(),
            first_seen_at: get_current_timestamp(),
            source_peer,
        };

        self.nodes.insert(block_hash, node);

        self.children_by_parent
            .entry(parent_hash)
            .or_default()
            .insert(block_hash);

        if let Some(parent_node) = self.nodes.get_mut(&parent_hash) {
            parent_node.children.insert(block_hash);
        }

        for child_hash in existing_children {
            if let Some(child_node) = self.nodes.get_mut(&child_hash) {
                child_node.parent = block_hash;
            }
        }
    }

    fn leaf_hashes(&self) -> Vec<BlockID> {
        self.nodes
            .iter()
            .filter_map(|(hash, node)| {
                if node.children.is_empty() {
                    Some(*hash)
                } else {
                    None
                }
            })
            .collect()
    }

    fn build_candidate_from_leaf(
        &self,
        blockchain: &Blockchain,
        leaf_hash: BlockID,
    ) -> Option<ReorgCandidate> {
        let mut blocks_reversed = Vec::new();
        let mut current_hash = leaf_hash;
        let mut visited = HashSet::new();

        loop {
            if !visited.insert(current_hash) {
                log_warning(
                    LogCategory::Core,
                    &format!(
                        "Detected cycle while evaluating fork tree at {}",
                        bytes_to_hex_string(&current_hash)
                    ),
                );
                return None;
            }

            let node = self.nodes.get(&current_hash)?;
            blocks_reversed.push(node.block.clone());

            let parent_hash = node.parent;

            if parent_hash == [0; 32] {
                blocks_reversed.reverse();

                return Some(ReorgCandidate {
                    ancestor_hash: [0; 32],
                    candidate_height: blocks_reversed.len(),
                    blocks: blocks_reversed,
                });
            }

            if let Some(ancestor_height) = blockchain.find_block_height_by_hash(parent_hash) {
                blocks_reversed.reverse();

                return Some(ReorgCandidate {
                    ancestor_hash: parent_hash,
                    candidate_height: ancestor_height + 1 + blocks_reversed.len(),
                    blocks: blocks_reversed,
                });
            }

            if !self.nodes.contains_key(&parent_hash) {
                return None;
            }

            current_hash = parent_hash;
        }
    }

    fn remove_node_only(&mut self, hash: BlockID) -> Option<Block> {
        let node = self.nodes.remove(&hash)?;

        self.detach_from_parent_index(node.parent, hash);
        self.children_by_parent.remove(&hash);

        for child in &node.children {
            if let Some(child_node) = self.nodes.get_mut(child) {
                child_node.parent = hash;
            }
            self.children_by_parent
                .entry(hash)
                .or_default()
                .insert(*child);
        }

        Some(node.block)
    }

    fn detach_from_parent_index(&mut self, parent_hash: BlockID, child_hash: BlockID) {
        if let Some(siblings) = self.children_by_parent.get_mut(&parent_hash) {
            siblings.remove(&child_hash);
            if siblings.is_empty() {
                self.children_by_parent.remove(&parent_hash);
            }
        }

        if let Some(parent_node) = self.nodes.get_mut(&parent_hash) {
            parent_node.children.remove(&child_hash);
        }
    }

    fn rebuild_children_index(&mut self) {
        self.children_by_parent.clear();

        for node in self.nodes.values_mut() {
            node.children.clear();
        }

        let edges: Vec<(BlockID, BlockID)> = self
            .nodes
            .iter()
            .map(|(hash, node)| (node.parent, *hash))
            .collect();

        for (parent_hash, child_hash) in edges {
            self.children_by_parent
                .entry(parent_hash)
                .or_default()
                .insert(child_hash);

            if let Some(parent_node) = self.nodes.get_mut(&parent_hash) {
                parent_node.children.insert(child_hash);
            }
        }
    }

    fn prune_to_capacity(&mut self) {
        while self.nodes.len() > self.max_blocks {
            let Some(oldest_hash) = self.oldest_node_hash() else {
                break;
            };

            log_warning(
                LogCategory::Core,
                &format!(
                    "Fork tree capacity exceeded ({} > {}). Pruning oldest subtree at {}",
                    self.nodes.len(),
                    self.max_blocks,
                    bytes_to_hex_string(&oldest_hash)
                ),
            );
            self.prune_subtree(oldest_hash);
        }
    }

    fn oldest_node_hash(&self) -> Option<BlockID> {
        self.nodes
            .iter()
            .min_by_key(|(_, node)| node.first_seen_at)
            .map(|(hash, _)| *hash)
    }
}
