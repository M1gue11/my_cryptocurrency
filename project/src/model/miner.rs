use std::collections::HashSet;

use crate::{
    globals::CONFIG,
    model::{Block, MempoolTx, Transaction, Wallet, transaction::TxId},
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

    fn get_legit_txs<'a>(&self, mempool: &'a Vec<MempoolTx>) -> Vec<&'a MempoolTx> {
        let mut seen_utxos: HashSet<(TxId, usize)> = HashSet::new();
        let mut selected_txs: Vec<&MempoolTx> = Vec::new();
        for mem_tx in mempool {
            let tx = &mem_tx.tx;
            let mut double_input = false;
            for input in &tx.inputs {
                if seen_utxos.contains(&(input.prev_tx_id, input.output_index)) {
                    double_input = true;
                    break;
                }
            }
            if double_input {
                continue;
            }
            seen_utxos.extend(
                tx.inputs
                    .iter()
                    .map(|input| (input.prev_tx_id, input.output_index)),
            );
            selected_txs.push(mem_tx);
        }
        selected_txs
    }

    fn build_block(&mut self, mempool: &Vec<MempoolTx>, previous_hash: [u8; 32]) -> Block {
        let mut txs = self.get_legit_txs(mempool);

        // Sort transactions descending by fee
        txs.sort_by(|a, b| {
            let fee_a: f64 = a.calculate_fee();
            let fee_b: f64 = b.calculate_fee();
            fee_b.partial_cmp(&fee_a).unwrap()
        });

        // Select transactions up to the maximum allowed per block
        // TODO:
        txs.truncate(CONFIG.max_txs_per_block);

        let total_fees: f64 = txs.iter().map(|mtx| mtx.calculate_fee()).sum();
        let mut block_txs: Vec<Transaction> = txs.iter().map(|mtx| mtx.tx.clone()).collect();
        let reward_tx = Transaction::new_coinbase(self.wallet.get_receive_addr(), total_fees);
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
        mempool: &Vec<MempoolTx>,
        previous_hash: [u8; 32],
        difficulty: usize,
    ) -> Block {
        let mut block_to_mine = self.build_block(mempool, previous_hash);
        self.mine_block(&mut block_to_mine, difficulty);
        block_to_mine
    }
}
