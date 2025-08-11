use super::{Block, Transaction};

#[derive(Debug)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    mempool: Vec<Transaction>,
}

impl Blockchain {
    pub fn new() -> Self {
        let mut chain = Vec::new();
        let genesis_block = Block::new("0".into());
        chain.push(genesis_block);

        Blockchain {
            chain,
            mempool: Vec::new(),
        }
    }

    pub fn get_mempool(&self) -> &Vec<Transaction> {
        &self.mempool
    }

    pub fn add_transaction_to_mempool(&mut self, t: Transaction) {
        self.mempool.push(t);
    }

    pub fn get_last_block_hash(&self) -> String {
        self.chain
            .last()
            .expect("Blockchain is empty")
            .calculate_hash()
    }

    pub fn add_block(&mut self, block: Block, difficulty: usize) -> bool {
        let last_block = self
            .chain
            .last()
            .expect("The chain should have at least one block.");
        let prefix = "0".repeat(difficulty);

        if block.prev_block_hash != last_block.calculate_hash() {
            println!("ERROR: Previous block hash does not match!");
            return false;
        }

        if !block.calculate_hash().starts_with(&prefix) {
            println!("ERROR: Invalid proof of work!");
            return false;
        }

        self.mempool
            .retain(|t| !block.transactions.iter().any(|btx| btx.id == t.id));
        self.chain.push(block);

        return true;
    }
}
