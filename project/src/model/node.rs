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
        println!("Starting a new node...");
        let bc = Blockchain::load_chain(None).unwrap_or_else(|_| Blockchain::new());

        if let Err(e) = Node::validate_blockchain(&bc) {
            panic!("Invalid blockchain data: {}", e);
        }

        if bc.is_empty() {
            println!("Blockchain is empty!");
        } else {
            println!("Loaded existing blockchain with {} blocks.", bc.chain.len());
        }
        Node {
            blockchain: bc,
            mempool: Node::load_mempool(),
            miner: Miner::new(),
            difficulty: CONSENSUS_RULES.difficulty,
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
                println!("No existing mempool file found.");
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

    pub fn is_chain_empty(&self) -> bool {
        self.blockchain.is_empty()
    }

    pub fn is_mempool_empty(&self) -> bool {
        self.mempool.is_empty()
    }

    pub fn validate_bc(&self) -> Result<bool, String> {
        Node::validate_blockchain(&self.blockchain)
    }

    /** Invalidate mempool transactions that are already included in the blockchain or are no longer valid */
    fn invalidate_mempool(&mut self) {
        let repo = LedgerRepository::new();
        self.mempool.retain(|tx| {
            if let Err(_) = repo.get_transaction(&tx.tx.id()) {
                true
            } else {
                false
            }
        });
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
                repo.apply_block(added_block.clone(), &added_block.transactions)
                    .map_err(|e| e.to_string())?;
                self.invalidate_mempool();
                Ok(())
            }
        }
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
        if repo.get_transaction(&tx.id()).is_ok() {
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

    pub fn print_mempool(&self) {
        println!("Mempool Transactions:");
        for mem_tx in &self.mempool {
            println!("{:#?}", mem_tx.tx);
        }
    }

    pub fn clear_mempool(&mut self) {
        self.mempool.clear();
    }

    pub fn get_node_version_info(&self) -> NodeVersion {
        NodeVersion {
            version: 1,
            height: self.blockchain.chain.len() as u64,
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
                    println!(
                        "Requesting block with ID: {}",
                        bytes_to_hex_string(&item_id)
                    );
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

    pub async fn handle_get_data_request(&self, item_type: InventoryType, item_id: [u8; 32]) {
        match item_type {
            InventoryType::Block => {
                if let Some(block) = self.blockchain.find_block_by_hash(item_id) {
                    network::send_block(block, None);
                } else {
                    println!(
                        "Requested block with ID {} not found.",
                        bytes_to_hex_string(&item_id)
                    );
                }
            }
            InventoryType::Tx => {
                if let Some(mem_tx) = self.get_mempool_tx_by_id(item_id) {
                    network::send_tx(&mem_tx.tx, None);
                } else {
                    let repo = LedgerRepository::new();
                    match repo.get_transaction(&item_id) {
                        Ok(tx) => {
                            network::send_tx(&tx, None);
                        }
                        Err(_) => {
                            println!(
                                "Requested transaction with ID {} not found.",
                                bytes_to_hex_string(&item_id)
                            );
                        }
                    }
                }
            }
        }
    }

    pub async fn handle_received_block(&mut self, block: Block, exclude_peer: Option<SocketAddr>) {
        if self.blockchain.find_block_by_hash(block.id()).is_some() {
            println!(
                "Block already exists in the blockchain. peer_addr: {:?}",
                exclude_peer
            );
            return;
        }
        let block_hash = block.id();
        match self.submit_block(block) {
            Ok(()) => {
                println!("Block added to the blockchain successfully.");
                network::broadcast_new_block_hash(block_hash, exclude_peer);
                // TODO: optimize persistence
                self.blockchain.persist_chain(None);
            }
            Err(e) => println!("Failed to add block to the blockchain: {}", e),
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
                println!("Failed to get UTXOs for transaction: {}", e);
                return;
            }
        };
        let tx_id = tx.id();
        match self.receive_transaction(MempoolTx { tx, utxos }) {
            Ok(()) => {
                println!("Transaction added to mempool successfully.");
                network::broadcast_new_tx_hash(tx_id, exclude_peer);
            }
            Err(e) => println!("Failed to add transaction to mempool: {}", e),
        }
    }

    pub async fn handle_get_blocks_request(&self, last_known_hash: [u8; 32]) {
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
            network::send_block(&block, None);
        }
    }

    pub async fn handle_version_message(&self, peer_v: NodeVersion) {
        let node_v = self.get_node_version_info();
        if peer_v.height > node_v.height {
            network::ask_for_blocks(self.blockchain.get_last_block_hash());
        } else if peer_v.height == node_v.height && peer_v.top_hash != node_v.top_hash {
            println!("Warning: Detected potential fork with peer node.");
            // TODO: Handle fork resolution
        }
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
