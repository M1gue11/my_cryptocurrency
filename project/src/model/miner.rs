use crate::{
    model::{Block, Transaction, Wallet},
    security_utils::hash_starts_with_zero_bits,
};

pub struct Miner {
    pub wallet: Wallet,
}

impl Miner {
    pub fn new() -> Self {
        Miner {
            wallet: Wallet::new("seed do miguel!"),
        }
    }

    fn build_block(&mut self, transactions: &Vec<Transaction>, previous_hash: [u8; 32]) -> Block {
        let mut block_txs = transactions.to_vec();

        let reward_tx = Transaction::new_coinbase(self.wallet.get_receive_addr());
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
        &mut self,
        mempool: &Vec<Transaction>,
        previous_hash: [u8; 32],
        difficulty: usize,
    ) -> Block {
        let mut block_to_mine = self.build_block(mempool, previous_hash);
        self.mine_block(&mut block_to_mine, difficulty);
        block_to_mine
    }
}
