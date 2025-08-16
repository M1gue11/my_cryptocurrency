use crate::config::CONFIG;
use crate::model::{Block, Blockchain, Miner, Transaction};

pub struct Node {
    blockchain: Blockchain,
    mempool: Vec<Transaction>,
    miner: Miner,
    difficulty: usize,
}

impl Node {
    pub fn new() -> Self {
        println!("Starting a new node...");
        Node {
            blockchain: Blockchain::new(),
            mempool: Vec::new(),
            miner: Miner::new(CONFIG.node_name.to_string()),
            difficulty: CONFIG.difficulty,
        }
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
        let last_block_hash = self.blockchain.get_last_block_hash();
        let mined_block = self
            .miner
            .mine(&self.mempool, last_block_hash, self.difficulty);

        self.submit_block(mined_block);

        self.blockchain
            .chain
            .last()
            .expect("Blockchain should not be empty")
    }

    pub fn save_node(&self) {
        // TODO: implement node saving
        self.blockchain.persist_chain(None);
    }

    pub fn print_chain(&self) {
        println!("{:#?}", self.blockchain);
    }
}
