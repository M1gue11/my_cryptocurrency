use crate::config::CONFIG;
use crate::model::{Block, Blockchain, Miner, Transaction};

const BLOCK_REWARD: f64 = 100.0;

pub struct Node {
    blockchain: Blockchain,
    mempool: Vec<Transaction>,
    miner: Miner,
    difficulty: usize,
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
            miner: Miner::new(CONFIG.node_name.to_string()),
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
            self.mempool
                .retain(|tx| !added_block.transactions.iter().any(|btx| btx.id == tx.id));
            true
        } else {
            false
        }
    }

    pub fn receive_transaction(&mut self, tx: Transaction) {
        // TODO: validate transaction
        self.mempool.push(tx);
    }

    pub fn mine(&mut self) -> &Block {
        let previous_hash = self.blockchain.get_last_block_hash();

        let mined_block =
            self.miner
                .mine(&self.mempool, previous_hash, self.difficulty, BLOCK_REWARD);

        self.submit_block(mined_block);
        self.blockchain.chain.last().unwrap()
    }

    pub fn save_node(&self) {
        // TODO: implement node saving
        self.blockchain.persist_chain(None);
    }

    pub fn print_chain(&self) {
        println!("{:#?}", self.blockchain);
    }
}
