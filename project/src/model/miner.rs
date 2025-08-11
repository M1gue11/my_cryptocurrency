use crate::model::{Block, Transaction};

pub struct Miner {
    pub mining_address: String,
}

impl Miner {
    pub fn new(mining_address: String) -> Self {
        Miner { mining_address }
    }

    fn build_block(&self, transactions: &Vec<Transaction>, previous_hash: String) -> Block {
        // TODO: build a logic to choose transactions
        let mut block_txs = transactions.to_vec();

        let reward_tx = Transaction::new(50.0, self.mining_address.clone(), "coinbase".into());
        block_txs.push(reward_tx);

        let mut new_block = Block::new(previous_hash);
        new_block.transactions = block_txs;
        new_block
    }

    fn mine_block(&self, block: &mut Block, difficulty: usize) {
        let prefix = "0".repeat(difficulty);

        while !Miner::is_hash_valid(&block.calculate_hash(), &prefix) {
            block.nonce += 1;
        }

        println!(
            "Bloco minerado! Nonce: {} Hash: {}",
            block.nonce,
            block.calculate_hash()
        );
    }

    pub fn mine(&self, transactions: &Vec<Transaction>, previous_hash: String, difficulty: usize) {
        let mut block_to_mine = self.build_block(transactions, previous_hash);
        self.mine_block(&mut block_to_mine, difficulty);
        println!(
            "Block mined successfully! Calculated nonce: {:?}",
            block_to_mine.nonce
        );
    }

    fn is_hash_valid(hash: &str, prefix: &str) -> bool {
        hash.starts_with(prefix)
    }
}
