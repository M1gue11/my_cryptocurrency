use crate::{
    model::{Block, Transaction},
    security_utils::hash_starts_with_zero_bits,
};

pub struct Miner {
    pub mining_address: String,
}

impl Miner {
    pub fn new(mining_address: String) -> Self {
        Miner { mining_address }
    }

    fn build_block(
        &self,
        transactions: &Vec<Transaction>,
        previous_hash: [u8; 32],
        block_reward: f64,
    ) -> Block {
        // TODO: build a logic to choose transactions
        let mut block_txs = transactions.to_vec();

        let reward_tx = Transaction::new_coinbase(block_reward, self.mining_address.clone());
        block_txs.insert(0, reward_tx);

        let mut new_block = Block::new(previous_hash);
        new_block.transactions = block_txs;
        new_block
    }

    fn mine_block(&self, block: &mut Block, difficulty: usize) {
        while !hash_starts_with_zero_bits(&block.calculate_hash(), difficulty) {
            block.nonce += 1;
        }
    }

    pub fn mine(
        &self,
        mempool: &Vec<Transaction>,
        previous_hash: [u8; 32],
        difficulty: usize,
        block_reward: f64,
    ) -> Block {
        // TODO: implement logic to decide which transactions to include
        let mut block_to_mine = self.build_block(mempool, previous_hash, block_reward);
        self.mine_block(&mut block_to_mine, difficulty);
        block_to_mine
    }
}
