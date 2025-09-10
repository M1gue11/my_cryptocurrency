use crate::globals::CONFIG;
use crate::model::io::UTXO;
use crate::model::{Block, Blockchain, Miner, Transaction};

pub struct Node {
    pub miner: Miner,
    blockchain: Blockchain,
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

    pub fn validate_blockchain(&self) -> bool {
        let chain_ref = &self.blockchain.chain;
        chain_ref.iter().enumerate().all(|(i, block)| {
            if i == 0 {
                return block.prev_block_hash == [0; 32];
            }
            let prev_block = &chain_ref[i - 1];
            block.prev_block_hash == prev_block.calculate_hash()
        })
    }

    fn submit_block(&mut self, block: Block) -> bool {
        if self.blockchain.add_block(block, self.difficulty) {
            // cleaning mempool
            let added_block = self.blockchain.chain.last().unwrap();

            // TODO: improve this logic to be more efficient
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

    pub fn receive_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        if !tx.validate() {
            return Err("Invalid transaction signature".to_string());
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
        // TODO: improve node saving
        self.blockchain.persist_chain(None);
    }

    pub fn print_chain(&self) {
        println!("{:#?}", self.blockchain);
    }
}
