use std::collections::HashSet;

use crate::{
    model::{Block, Transaction, Wallet, transaction::TxId},
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

    fn chose_transactions(&self, mempool: &Vec<Transaction>) -> Vec<Transaction> {
        let mut seen_utxos: HashSet<(TxId, usize)> = HashSet::new();
        let mut selected_txs: Vec<Transaction> = Vec::new();
        for tx in mempool {
            for input in &tx.inputs {
                let key = (input.prev_tx_id, input.output_index);
                if seen_utxos.contains(&key) {
                    continue;
                }
                seen_utxos.insert(key);
                selected_txs.push(tx.clone());
            }
        }
        selected_txs
    }

    fn build_block(&mut self, mempool: &Vec<Transaction>, previous_hash: [u8; 32]) -> Block {
        let mut block_txs = self.chose_transactions(mempool);

        let reward_tx = Transaction::new_coinbase(self.wallet.get_receive_addr());
        block_txs.insert(0, reward_tx);

        let mut new_block = Block::new(previous_hash);
        new_block.transactions = block_txs;
        new_block.evaluate_merkle_root();
        new_block
    }

    fn mine_block(&self, block: &mut Block, difficulty: usize) {
        while !hash_starts_with_zero_bits(&block.header_hash(), difficulty) {
            block.header.nonce += 1;
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
