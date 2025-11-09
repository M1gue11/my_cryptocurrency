use std::collections::HashSet;

use crate::globals::CONFIG;
use crate::model::io::UTXO;
use crate::model::transaction::TxId;
use crate::model::{Block, Blockchain, Miner, Transaction};
use crate::security_utils::digest_to_hex_string;

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
            mempool: Vec::new(),
            miner: Miner::new(),
            difficulty: CONFIG.difficulty,
        }
    }

    pub fn is_chain_empty(&self) -> bool {
        self.blockchain.is_empty()
    }

    pub fn validate_blockchain(bc: &Blockchain) -> Result<bool, String> {
        let chain_ref = &bc.chain;
        for (i, block) in chain_ref.iter().enumerate() {
            if i == 0 {
                if block.header.prev_block_hash != [0; 32] {
                    println!("OIIII");
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

    fn submit_block(&mut self, block: Block) -> bool {
        if self.blockchain.add_block(block) {
            // cleaning mempool
            let added_block = self.blockchain.chain.last().unwrap();

            self.mempool.retain(|tx| {
                !added_block
                    .transactions
                    .iter()
                    .any(|btx| btx.id() == tx.id())
            });
            true
        } else {
            false
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

    pub fn mine(&mut self) -> &Block {
        let previous_hash = self.blockchain.get_last_block_hash();

        let mined_block = self
            .miner
            .mine(&self.mempool, previous_hash, self.difficulty);

        self.submit_block(mined_block);
        self.blockchain.chain.last().unwrap()
    }

    pub fn save_node(&self) {
        self.blockchain.persist_chain(None);
    }

    pub fn print_chain(&self) {
        println!("{:#?}", self.blockchain);
    }
}
