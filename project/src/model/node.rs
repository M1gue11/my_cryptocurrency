use std::collections::HashSet;
use std::fs::File;
use std::io::BufWriter;

use crate::bd::Db;
use crate::globals::CONFIG;
use crate::model::io::UTXO;
use crate::model::transaction::TxId;
use crate::model::{Block, Blockchain, Miner, Transaction};
use crate::security_utils::digest_to_hex_string;

const MEMPOOL_FILE: &str = "mempool.json";

pub struct Node {
    pub miner: Miner,
    pub blockchain: Blockchain,
    mempool: Vec<Transaction>,
    difficulty: usize,
}

static mut NODE: Option<Node> = None;

pub fn init_node() {
    unsafe {
        NODE = Some(Node::new());
    }
}

#[allow(static_mut_refs)]
pub fn get_node_mut() -> &'static mut Node {
    unsafe { NODE.as_mut().expect("Node não inicializado") }
}

#[allow(static_mut_refs)]
pub fn get_node() -> &'static Node {
    unsafe { NODE.as_ref().expect("Node não inicializado") }
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
            difficulty: CONFIG.difficulty,
        }
    }

    fn persist_mempool(&self) {
        let path = CONFIG.persisted_chain_path.to_string();
        let dir_path = std::path::Path::new(&path);
        if !dir_path.exists() {
            std::fs::create_dir_all(&dir_path)
                .expect("Failed to create directory for mempool file");
        }

        let file = File::create(format!("{}/{}", path, MEMPOOL_FILE))
            .expect("Failed to create blockchain file");
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &self.mempool).expect("Failed to write blockchain to file");
    }

    fn load_mempool() -> Vec<Transaction> {
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
        let mempool: Vec<Transaction> =
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
                    digest_to_hex_string(&block.id())
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

    fn submit_block(&mut self, block: Block) -> Result<(), String> {
        match self.blockchain.add_block(block) {
            Err(e) => return Err(e),
            Ok(()) => {
                let added_block = self.blockchain.chain.last().unwrap();

                self.mempool.retain(|tx| {
                    !added_block
                        .transactions
                        .iter()
                        .any(|btx| btx.id() == tx.id())
                });
                let mut db = Db::open(None).unwrap();
                db.apply_block(added_block.clone(), &added_block.transactions)
                    .map_err(|e| e.to_string())?;

                Ok(())
            }
        }
    }

    pub fn find_transaction(&self, tx_id: &[u8; 32]) -> Option<&Transaction> {
        for block in &self.blockchain.chain {
            for tx in &block.transactions {
                if &tx.id() == tx_id {
                    return Some(tx);
                }
            }
        }
        None
    }

    pub fn scan_utxos(&self) -> Vec<UTXO> {
        let mut utxos = Vec::new();
        for block in &self.blockchain.chain {
            for tx in &block.transactions {
                for (output_index, output) in tx.outputs.iter().enumerate() {
                    utxos.push(UTXO {
                        tx_id: tx.id(),
                        index: output_index,
                        output: output.clone(),
                    });
                }
                for input in &tx.inputs {
                    utxos.retain(|o| o.tx_id != input.prev_tx_id);
                }
            }
        }
        utxos
    }

    pub fn is_all_inputs_utxos(&self, tx: &Transaction) -> Result<(), String> {
        let utxos = self.scan_utxos();
        let utxos_map: HashSet<(TxId, usize)> =
            HashSet::from_iter(utxos.iter().map(|u| (u.tx_id, u.index)));
        for input in &tx.inputs {
            if !utxos_map.contains(&(input.prev_tx_id, input.output_index)) {
                return Err(format!(
                    "Transaction input is not a valid UTXO: tx_id: {}, output_index: {}",
                    digest_to_hex_string(&input.prev_tx_id),
                    input.output_index
                ));
            }
        }
        Ok(())
    }

    pub fn receive_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        if !tx.validate() {
            return Err("Invalid transaction signature".to_string());
        }
        if self
            .mempool
            .iter()
            .any(|mempool_tx| mempool_tx.id() == tx.id())
        {
            return Err("Transaction already in mempool".to_string());
        }

        if self.find_transaction(&tx.id()).is_some() {
            return Err("Transaction already in blockchain!".to_string());
        }

        if let Err(e) = self.is_all_inputs_utxos(&tx) {
            return Err(e);
        }

        self.mempool.push(tx);
        Ok(())
    }

    pub fn mine(&mut self) -> Result<&Block, String> {
        let previous_hash = self.blockchain.get_last_block_hash();

        let mined_block = self
            .miner
            .mine(&self.mempool, previous_hash, self.difficulty);

        match self.submit_block(mined_block) {
            Ok(()) => Ok(self.blockchain.chain.last().unwrap()),
            Err(e) => Err(e),
        }
    }

    pub fn save_node(&self) {
        self.blockchain.persist_chain(None);
        self.persist_mempool();
    }

    pub fn print_mempool(&self) {
        println!("Mempool Transactions:");
        for tx in &self.mempool {
            println!("{:#?}", tx);
        }
    }

    pub fn rollback_blocks(&mut self, count: u32) -> Result<(), String> {
        if count == 0 {
            return Err("Count must be greater than zero".to_string());
        }

        let chain_len = self.blockchain.chain.len();
        if chain_len == 0 {
            return Err("Blockchain is empty, nothing to rollback".to_string());
        }

        let blocks_to_remove = count as usize;
        if blocks_to_remove >= chain_len {
            return Err(format!(
                "Cannot rollback {} blocks. Only {} blocks in the chain",
                count, chain_len
            ));
        }

        // collect transactions from blocks to be removed
        let mut transactions_to_restore = Vec::new();
        for i in (chain_len - blocks_to_remove..chain_len).rev() {
            let block = &self.blockchain.chain[i];
            // skip coinbase transactions (they have no inputs)
            for tx in &block.transactions {
                if !tx.is_coinbase() {
                    transactions_to_restore.push(tx.clone());
                }
            }
        }

        // remove blocks from the chain
        self.blockchain.chain.truncate(chain_len - blocks_to_remove);

        // add transactions back to mempool
        for tx in transactions_to_restore {
            if !self
                .mempool
                .iter()
                .any(|mempool_tx| mempool_tx.id() == tx.id())
            {
                self.mempool.push(tx);
            }
        }

        Ok(())
    }
}
