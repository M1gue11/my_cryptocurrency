use std::collections::HashSet;
use std::fs::File;
use std::io::BufWriter;
use std::net::SocketAddr;
use std::sync::Arc;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::db::repository::LedgerRepository;
use crate::globals::{CONFIG, CONSENSUS_RULES};
use crate::model::transaction::TxId;
use crate::model::{Block, Blockchain, MempoolTx, Miner, Transaction};
use crate::network::get_peer_count;
use crate::network::network_message::InventoryType;
use crate::security_utils::bytes_to_hex_string;
use crate::{network, utils};

const MEMPOOL_FILE: &str = "mempool.json";

pub struct Node {
    pub miner: Miner,
    pub blockchain: Blockchain,
    mempool: Vec<MempoolTx>,
    difficulty: usize,
    fork_helper: utils::ForkHelper,
}

pub static NODE: Lazy<Arc<RwLock<Node>>> = Lazy::new(|| Arc::new(RwLock::new(Node::new())));

pub async fn get_node() -> tokio::sync::RwLockReadGuard<'static, Node> {
    NODE.read().await
}

pub async fn get_node_mut() -> tokio::sync::RwLockWriteGuard<'static, Node> {
    NODE.write().await
}

pub async fn restart_node() {
    let mut node_guard = NODE.write().await;
    *node_guard = Node::new();
}

impl Node {
    pub fn new() -> Self {
        utils::log_info(utils::LogCategory::Core, "Starting a new node...");
        let bc = Blockchain::load_chain(None).unwrap_or_else(|_| Blockchain::new());

        if let Err(e) = Node::validate_blockchain(&bc) {
            panic!("Invalid blockchain data: {}", e);
        }

        if bc.is_empty() {
            utils::log_info(utils::LogCategory::Core, "Blockchain is empty!");
        } else {
            utils::log_info(utils::LogCategory::Core, &format!("Loaded existing blockchain with {} blocks.", bc.chain.len()));
        }
        Node {
            blockchain: bc,
            mempool: Node::load_mempool(),
            miner: Miner::new(),
            difficulty: CONSENSUS_RULES.difficulty,
            fork_helper: utils::ForkHelper::new(),
        }
    }

    pub fn persist_mempool(&self) {
        let path = CONFIG.persisted_chain_path.to_string();
        utils::assert_parent_dir_exists(&path)
            .expect("Failed to create parent directories for blockchain file");

        let file = File::create(format!("{}/{}", path, MEMPOOL_FILE))
            .expect("Failed to create blockchain file");
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &self.mempool).expect("Failed to write blockchain to file");
    }

    fn load_mempool() -> Vec<MempoolTx> {
        let path = CONFIG.persisted_chain_path.to_string();
        let file_path = format!("{}/{}", path, MEMPOOL_FILE);

        let file = match File::open(&file_path) {
            Ok(f) => f,
            Err(_) => {
                utils::log_info(utils::LogCategory::Core, "No existing mempool file found.");
                return Vec::new();
            }
        };
        let reader = std::io::BufReader::new(file);
        let mempool: Vec<MempoolTx> =
            serde_json::from_reader(reader).expect("Failed to read mempool from file");
        mempool
    }

    fn validate_blockchain(bc: &Blockchain) -> Result<bool, String> {
        let chain_ref = &bc.chain;
        for (i, block) in chain_ref.iter().enumerate() {
            if i == 0 {
                if block.header.prev_block_hash != [0; 32] {
                    return Err("Genesis block has invalid previous hash".to_string());
                }
                continue;
            }
            let prev_block = &chain_ref[i - 1];

            if let Err(e) = block.validate() {
                return Err(e);
            }

            if block.header.prev_block_hash != prev_block.header_hash() {
                return Err(format!(
                    "Block {} has invalid previous block hash",
                    bytes_to_hex_string(&block.id())
                ));
            }
        }
        Ok(true)
    }

    pub fn get_mempool(&self) -> &Vec<MempoolTx> {
        &self.mempool
    }

    pub fn validate_bc(&self) -> Result<bool, String> {
        Node::validate_blockchain(&self.blockchain)
    }

    /** Invalidate mempool transactions that are already included in the blockchain or are no longer valid */
    fn invalidate_mempool(&mut self) {
        let repo = LedgerRepository::new();
        self.mempool
            .retain(|tx| !matches!(repo.get_transaction(&tx.tx.id()), Ok(Some(_))));
        let txs_to_remove: Vec<TxId> = self
            .mempool
            .iter()
            .enumerate()
            .filter_map(|(_, mem_tx)| {
                if let Err(_) = self.is_all_inputs_utxos(&mem_tx.tx) {
                    Some(mem_tx.tx.id())
                } else {
                    None
                }
            })
            .collect();
        let txs_to_remove_set: HashSet<TxId> = HashSet::from_iter(txs_to_remove);
        self.mempool
            .retain(|mem_tx| !txs_to_remove_set.contains(&mem_tx.tx.id()));
    }

    fn submit_block(&mut self, block: Block) -> Result<(), String> {
        match self.blockchain.add_block(block) {
            Err(e) => return Err(e),
            Ok(()) => {
                let added_block = self.blockchain.chain.last().unwrap();

                self.mempool.retain(|mem_tx| {
                    !added_block
                        .transactions
                        .iter()
                        .any(|btx| btx.id() == mem_tx.tx.id())
                });
                let mut repo = LedgerRepository::new();
                repo.apply_block(added_block.clone())
                    .map_err(|e| e.to_string())?;
                self.invalidate_mempool();
                Ok(())
            }
        }
    }

    /// Rollback the last block from the blockchain
    /// Returns the rolled back block and its transactions for potential re-addition to mempool
    pub fn rollback_last_block(&mut self) -> Result<(Block, Vec<Transaction>), String> {
        // Validate that there is a block to rollback (not genesis)
        if self.blockchain.height() == 0 {
            return Err("Cannot rollback genesis block".to_string());
        }

        // Get the last block before removing it
        let last_block = self
            .blockchain
            .chain
            .last()
            .ok_or("Blockchain is empty")?
            .clone();

        // Get transactions from the block (excluding coinbase)
        let transactions: Vec<Transaction> = last_block
            .transactions
            .iter()
            .filter(|tx| !tx.is_coinbase())
            .cloned()
            .collect();

        // Rollback database changes
        let mut repo = LedgerRepository::new();
        repo.rollback_block(&last_block)
            .map_err(|e| format!("Database rollback failed: {}", e))?;

        // Remove block from blockchain
        self.blockchain.chain.pop();

        // Re-add non-coinbase transactions to mempool
        let repo = LedgerRepository::new();
        for tx in &transactions {
            let utxo_ids: Vec<_> = tx
                .inputs
                .iter()
                .map(|i| (i.prev_tx_id, i.output_index))
                .collect();
            let utxos = repo.get_utxos_from_ids(&utxo_ids).unwrap_or_default();
            self.mempool.push(MempoolTx::new(tx.clone(), utxos));
        }

        // Invalidate mempool to revalidate all transactions
        self.invalidate_mempool();
        utils::log_info(utils::LogCategory::Core, &format!(
            "Rolled back block {} at height {}",
            bytes_to_hex_string(&last_block.id()),
            self.blockchain.height() + 1
        ));

        Ok((last_block, transactions))
    }

    /// Rollback multiple blocks until reaching the specified block hash
    /// Returns all rolled back blocks and their transactions
    pub fn rollback_to_block(
        &mut self,
        target_block_hash: &[u8; 32],
    ) -> Result<Vec<(Block, Vec<Transaction>)>, String> {
        let mut rolled_back_blocks = Vec::new();

        // Keep rolling back until we reach the target block
        loop {
            // Check if current top block is the target
            let current_top = self.blockchain.chain.last().ok_or("Blockchain is empty")?;

            if current_top.id() == *target_block_hash {
                break;
            }

            // Rollback one block
            let (block, txs) = self.rollback_last_block()?;
            rolled_back_blocks.push((block, txs));
        }

        if rolled_back_blocks.is_empty() {
            utils::log_info(utils::LogCategory::Core, "No blocks needed to be rolled back");
        } else {
            utils::log_info(utils::LogCategory::Core, &format!(
                "Successfully rolled back {} blocks to reach block {}",
                rolled_back_blocks.len(),
                bytes_to_hex_string(target_block_hash)
            ));
        }

        Ok(rolled_back_blocks)
    }

    pub fn rebase_chain_to_fork(&mut self, fork: utils::Fork, peer_addr: Option<SocketAddr>) {
        utils::log_info(utils::LogCategory::Core, &format!(
            "Starting rebase to fork with starting block {} and length {}",
            bytes_to_hex_string(fork.get_fork_start().unwrap()),
            fork.blocks_sequence.len()
        ));
        let _ = match self.rollback_to_block(fork.get_fork_start().unwrap()) {
            Ok(rb) => rb,
            Err(e) => {
                utils::log_error(utils::LogCategory::Core, &format!("Failed to rollback to fork start: {}", e));
                return;
            }
        };
        // Clear all forks after a successful rebase -- old tracking data is invalid
        self.fork_helper.clear_forks();
        network::ask_for_blocks(self.blockchain.get_last_block_hash(), peer_addr);
    }

    pub fn is_all_inputs_utxos(&self, tx: &Transaction) -> Result<(), String> {
        let repo = LedgerRepository::new();
        let inputs_ids = tx
            .inputs
            .iter()
            .map(|i| (i.prev_tx_id, i.output_index))
            .collect::<Vec<_>>();
        let utxos = repo
            .get_utxos_from_ids(&inputs_ids)
            .map_err(|e| return e.to_string())?;

        if utxos.len() != inputs_ids.len() {
            return Err("One or more transaction inputs are not valid UTXOs".to_string());
        }

        let valid_utxos_map: HashSet<([u8; 32], usize)> =
            HashSet::from_iter(utxos.iter().map(|u| (u.tx_id, u.index)));
        for (txid, vout) in inputs_ids {
            if !valid_utxos_map.contains(&(txid, vout)) {
                return Err(format!(
                    "Transaction input is not a valid UTXO: tx_id: {}, output_index: {}",
                    bytes_to_hex_string(&txid),
                    vout
                ));
            }
        }
        Ok(())
    }

    pub fn receive_transaction(&mut self, mem_txs: MempoolTx) -> Result<(), String> {
        let tx = &mem_txs.tx;
        if let Err(e) = tx.validate() {
            return Err(e.to_string());
        }
        if self
            .mempool
            .iter()
            .any(|mempool_tx| mempool_tx.tx.id() == tx.id())
        {
            return Err("Transaction already in mempool".to_string());
        }

        let repo = LedgerRepository::new();
        if matches!(repo.get_transaction(&tx.id()), Ok(Some(_))) {
            return Err("Transaction already in blockchain!".to_string());
        }

        if let Err(e) = self.is_all_inputs_utxos(&tx) {
            return Err(e);
        }
        network::broadcast_new_tx_hash(tx.id(), None);
        self.mempool.push(mem_txs);
        Ok(())
    }

    pub fn mine(&mut self) -> Result<&Block, String> {
        let previous_hash = self.blockchain.get_last_block_hash();

        let mined_block = self
            .miner
            .mine(&self.mempool, previous_hash, self.difficulty)?;

        match self.submit_block(mined_block) {
            Ok(()) => {
                let new_block = self.blockchain.chain.last().unwrap();
                network::broadcast_new_block_hash(new_block.id(), None);
                Ok(new_block)
            }
            Err(e) => Err(e),
        }
    }

    pub fn save_node(&self) {
        self.blockchain.persist_chain(None);
        self.persist_mempool();
    }

    pub fn clear_mempool(&mut self) {
        self.mempool.clear();
    }

    pub fn get_node_version_info(&self) -> NodeVersion {
        NodeVersion {
            version: 1,
            height: self.blockchain.height() as u64,
            top_hash: self.blockchain.get_last_block_hash(),
        }
    }

    pub async fn get_node_state(&self) -> NodeState {
        let v = self.get_node_version_info();
        let peers_count = get_peer_count().await;
        return NodeState {
            version: v,
            peers_connected: peers_count,
        };
    }

    pub fn get_mempool_tx_by_id(&self, tx_id: [u8; 32]) -> Option<&MempoolTx> {
        self.mempool.iter().find(|mtx| mtx.tx.id() == tx_id)
    }

    pub async fn handle_inventory(
        &self,
        items: Vec<(InventoryType, [u8; 32])>,
        _exclude_peer: Option<SocketAddr>,
    ) {
        for (inv_type, item_id) in items {
            match inv_type {
                InventoryType::Block => {
                    if self.blockchain.find_block_by_hash(item_id).is_some() {
                        continue;
                    }
                    utils::log_info(utils::LogCategory::P2P, &format!(
                        "Requesting block with ID: {}",
                        bytes_to_hex_string(&item_id)
                    ));
                    network::ask_for_block(item_id);
                }
                InventoryType::Tx => {
                    if self.get_mempool_tx_by_id(item_id).is_some() {
                        continue;
                    }
                    network::ask_for_tx(item_id);
                }
            }
        }
    }

    pub async fn handle_get_data_request(
        &self,
        item_type: InventoryType,
        item_id: [u8; 32],
        requester: Option<SocketAddr>,
    ) {
        match item_type {
            InventoryType::Block => {
                if let Some(block) = self.blockchain.find_block_by_hash(item_id) {
                    if let Some(peer) = requester {
                        network::send_block_to(block, peer);
                    } else {
                        utils::log_warning(utils::LogCategory::P2P, "Requested peer is None, not sending block.");
                    }
                } else {
                    utils::log_warning(utils::LogCategory::Core, &format!(
                        "Requested block with ID {} not found.",
                        bytes_to_hex_string(&item_id)
                    ));
                }
            }
            InventoryType::Tx => {
                if let Some(mem_tx) = self.get_mempool_tx_by_id(item_id) {
                    if let Some(peer) = requester {
                        network::send_tx_to(&mem_tx.tx, peer);
                    } else {
                        utils::log_warning(utils::LogCategory::P2P, "Requested peer is None, not sending transaction.");
                    }
                } else {
                    let repo = LedgerRepository::new();
                    match repo.get_transaction(&item_id) {
                        Ok(Some(tx)) => {
                            if let Some(peer) = requester {
                                network::send_tx_to(&tx, peer);
                            } else {
                                utils::log_warning(utils::LogCategory::P2P, "Requested peer is None, not sending transaction.");
                            }
                        }
                        _ => {
                            utils::log_warning(utils::LogCategory::Core, &format!(
                                "Requested transaction with ID {} not found.",
                                bytes_to_hex_string(&item_id)
                            ));
                        }
                    }
                }
            }
        }
    }

    pub async fn handle_received_block(&mut self, block: Block, exclude_peer: Option<SocketAddr>) {
        if self.blockchain.find_block_by_hash(block.id()).is_some() {
            utils::log_info(utils::LogCategory::P2P, &format!(
                "Block already exists in the blockchain: {}. peer_addr: {:?}",
                bytes_to_hex_string(&block.id()),
                exclude_peer
            ));
            return;
        }

        if self.fork_helper.verify_fork(
            self.blockchain
                .get_last_block()
                .expect("Blockchain is empty"),
            &block,
        ) {
            utils::log_info(utils::LogCategory::P2P, &format!(
                "Received block {} from peer {:?} that creates or extends a fork.",
                bytes_to_hex_string(&block.id()),
                exclude_peer
            ));
            let new_bigger_branch = self.fork_helper.evaluate_forks(&self);
            if let Some(fork) = new_bigger_branch {
                self.rebase_chain_to_fork(fork, exclude_peer);
            }
            return;
        }

        let block_hash = block.id();
        match self.submit_block(block) {
            Ok(()) => {
                utils::log_info(utils::LogCategory::Core, &format!(
                    "Block {} added to the blockchain successfully.",
                    bytes_to_hex_string(&block_hash)
                ));
                network::broadcast_new_block_hash(block_hash, exclude_peer);
                // TODO: optimize persistence
                self.blockchain.persist_chain(None);
            }
            Err(e) => utils::log_error(utils::LogCategory::Core, &format!("Failed to add block to the blockchain: {}", e)),
        }
    }

    pub async fn handle_received_transaction(
        &mut self,
        tx: Transaction,
        exclude_peer: Option<SocketAddr>,
    ) {
        let utxos_ids = tx
            .inputs
            .iter()
            .map(|i| (i.prev_tx_id, i.output_index))
            .collect::<Vec<_>>();
        let repo = LedgerRepository::new();
        let utxos = match repo.get_utxos_from_ids(&utxos_ids) {
            Ok(u) => u,
            Err(e) => {
                utils::log_error(utils::LogCategory::Core, &format!("Failed to get UTXOs for transaction: {}", e));
                return;
            }
        };
        let tx_id = tx.id();
        match self.receive_transaction(MempoolTx { tx, utxos }) {
            Ok(()) => {
                utils::log_info(utils::LogCategory::Core, "Transaction added to mempool successfully.");
                network::broadcast_new_tx_hash(tx_id, exclude_peer);
            }
            Err(e) => utils::log_error(utils::LogCategory::Core, &format!("Failed to add transaction to mempool: {}", e)),
        }
    }

    pub async fn handle_get_blocks_request(
        &self,
        last_known_hash: [u8; 32],
        target_peer: SocketAddr,
    ) {
        let mut blocks_to_send = Vec::new();
        let mut found = false;

        if last_known_hash == [0; 32] {
            found = true;
        }
        for block in &self.blockchain.chain {
            if found {
                blocks_to_send.push(block);
            } else if block.header_hash() == last_known_hash {
                found = true;
            }
        }

        // TODO: improve this
        for block in blocks_to_send {
            network::send_block_to(&block, target_peer);
        }
    }

    pub async fn handle_version_message(&self, peer_v: NodeVersion, peer_addr: Option<SocketAddr>) {
        let node_v = self.get_node_version_info();
        let peer = match peer_addr {
            Some(addr) => addr,
            None => {
                utils::log_warning(utils::LogCategory::P2P, "Peer address is None, cannot log version info.");
                return;
            }
        };
        if node_v.height == peer_v.height {
            if node_v.top_hash != peer_v.top_hash {
                utils::log_warning(utils::LogCategory::P2P, "Peer has same height but different top hash.");
                utils::log_info(utils::LogCategory::P2P, "This could indicate a fork. Requesting blocks to find common ancestor...");
                network::find_common_ancestor(self.blockchain.build_block_sequence(), peer);
            }
        } else if peer_v.height > node_v.height {
            utils::log_info(utils::LogCategory::P2P, "Peer has a longer chain. Requesting blocks...");
            network::find_common_ancestor(self.blockchain.build_block_sequence(), peer);
        }
    }

    pub async fn handle_find_common_ancestor_request(
        &self,
        peer_blocks_hashes: Vec<[u8; 32]>,
        target_peer: SocketAddr,
    ) {
        for hash in peer_blocks_hashes.iter().rev() {
            let block = self.blockchain.find_block_by_hash(*hash);
            if block.is_some() {
                network::send_common_block(&block.unwrap(), target_peer);
                return;
            }
        }
        utils::log_warning(utils::LogCategory::P2P, &format!("No common ancestor found with peer {}", target_peer));
    }

    pub async fn handle_received_common_block(
        &mut self,
        block: Block,
        peer_addr: Option<SocketAddr>,
    ) {
        let block_hash = block.id();

        if self.blockchain.find_block_by_hash(block_hash).is_none() {
            utils::log_warning(utils::LogCategory::P2P, &format!(
                "Received common block {} from peer {:?} but it's not in our blockchain!",
                bytes_to_hex_string(&block_hash),
                peer_addr
            ));
            return;
        }

        // If the common block is our chain tip, there is no fork
        if let Some(last_block) = self.blockchain.get_last_block() {
            if last_block.id() == block_hash {
                utils::log_info(utils::LogCategory::P2P, &format!(
                    "Common block {} is our chain tip -- no fork.",
                    bytes_to_hex_string(&block_hash)
                ));
                return;
            }
        }

        // Register this as a fork starting point and request diverging blocks
        self.fork_helper.register_fork_start(block_hash);
        utils::log_info(utils::LogCategory::P2P, &format!(
            "Received common block {} from peer {:?}. Fork registered, requesting blocks.",
            bytes_to_hex_string(&block_hash),
            peer_addr
        ));
        network::ask_for_blocks(block_hash, peer_addr);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeVersion {
    pub version: u32,
    pub height: u64,
    pub top_hash: [u8; 32],
}

pub struct NodeState {
    pub version: NodeVersion,
    pub peers_connected: usize,
}
