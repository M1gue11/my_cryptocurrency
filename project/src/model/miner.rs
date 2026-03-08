use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tokio::task::JoinSet;

use primitive_types::U256;

use crate::utils::log_info;
use crate::{
    globals::{CONFIG, CONSENSUS_RULES},
    model::{Block, MempoolTx, Transaction, Wallet, transaction::TxId},
    security_utils::hash_meets_target,
    utils,
};

pub struct Miner {
    pub wallet: Wallet,
}

impl Miner {
    pub fn new() -> Self {
        let wallet = match Wallet::from_keystore_file(
            &CONFIG.miner_wallet_seed_path,
            &CONFIG.miner_wallet_password,
        ) {
            Ok(w) => w,
            Err(e) => {
                utils::log_error(
                    utils::LogCategory::Core,
                    &format!(
                        "Failed to load miner wallet! Path: {} - Error: {}",
                        CONFIG.miner_wallet_seed_path, e
                    ),
                );
                std::process::exit(1);
            }
        };
        Miner { wallet }
    }
}

fn get_legit_txs<'a>(mempool: &'a Vec<MempoolTx>) -> Vec<&'a MempoolTx> {
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

fn build_block(
    mempool: &Vec<MempoolTx>,
    previous_hash: [u8; 32],
    target: U256,
    receive_addr: &str,
) -> Block {
    let mut txs = get_legit_txs(mempool);
    // Sort transactions descending by fee_rate
    txs.sort_by(|a, b| {
        let fee_rate_a: f64 = a.calculate_fee_per_byte();
        let fee_rate_b: f64 = b.calculate_fee_per_byte();
        fee_rate_b.partial_cmp(&fee_rate_a).unwrap()
    });
    let estimated_coinbase_tx_size = Transaction::new_coinbase(receive_addr.to_string(), 0)
        .as_bytes()
        .len();
    let max_block_size_bytes =
        (CONSENSUS_RULES.max_block_size_kb * 1000.0) as usize - estimated_coinbase_tx_size;
    let mut cutoff_index = 0;
    let mut curr_block_size_bytes = 0;
    for (i, mtx) in txs.iter().enumerate() {
        let tx_size = mtx.tx.as_bytes().len();
        if curr_block_size_bytes + tx_size > max_block_size_bytes {
            break;
        }
        curr_block_size_bytes += tx_size;
        cutoff_index = i;
    }
    utils::log_info(
        utils::LogCategory::Core,
        &format!(
            "Current block size (bytes): {} / {}",
            curr_block_size_bytes, max_block_size_bytes
        ),
    );
    utils::log_info(
        utils::LogCategory::Core,
        &format!("Cutoff index: {}", cutoff_index),
    );
    txs.truncate(cutoff_index + 1);

    utils::log_info(
        utils::LogCategory::Core,
        &format!("Transactions selected for block: {}", txs.len()),
    );

    let total_fees: i64 = txs.iter().map(|mtx| mtx.calculate_fee()).sum();
    let mut block_txs: Vec<Transaction> = txs.iter().map(|mtx| mtx.tx.clone()).collect();
    let reward_tx = Transaction::new_coinbase(receive_addr.to_string(), total_fees);
    block_txs.insert(0, reward_tx);

    let mut new_block = Block::new(previous_hash, target);
    new_block.transactions = block_txs;
    new_block.evaluate_merkle_root();
    new_block
}

pub async fn mine_block(
    mempool: Vec<MempoolTx>,
    previous_hash: [u8; 32],
    target: U256,
    receive_addr: String,
) -> Result<Block, String> {
    let threads = CONFIG.mining_threads.max(1);
    let mut attempts = 0;
    loop {
        if attempts >= CONFIG.max_mining_attempts {
            return Err(format!(
                "Failed to mine block: exceeded maximum attempts ({})",
                CONFIG.max_mining_attempts
            ));
        }

        let block_template = build_block(&mempool, previous_hash, target, &receive_addr);
        let found = Arc::new(AtomicBool::new(false));
        let range_size = u32::MAX / threads as u32;

        let mut set: JoinSet<Option<Block>> = JoinSet::new();
        for i in 0..threads {
            let mut block = block_template.clone();
            let found = Arc::clone(&found);
            let nonce_start = i as u32 * range_size;
            let nonce_end = if i + 1 == threads {
                u32::MAX
            } else {
                nonce_start + range_size - 1
            };
            log_info(
                utils::LogCategory::Core,
                &format!(
                    "Thread {} mining block with nonce range: {} - {}",
                    i, nonce_start, nonce_end
                ),
            );
            set.spawn_blocking(move || {
                block.header.nonce = nonce_start;
                let start = std::time::Instant::now();
                let log_interval = 500_000u32;
                let mut last_log_nonce = nonce_start;
                loop {
                    if found.load(Ordering::Relaxed) {
                        return None;
                    }
                    if hash_meets_target(&block.header_hash(), &block.header.target) {
                        found.store(true, Ordering::Relaxed);
                        return Some(block);
                    }
                    if block.header.nonce >= nonce_end {
                        return None;
                    }
                    block.header.nonce += 1;
                    // logging progress every log_interval nonces
                    if block.header.nonce - last_log_nonce >= log_interval {
                        let elapsed = start.elapsed().as_secs_f64();
                        let hashes = (block.header.nonce - nonce_start) as f64;
                        let hash_rate = hashes / elapsed;
                        utils::log_info(
                            utils::LogCategory::Core,
                            &format!(
                                "[miner thread {}] nonce={} | {:.0} h/s | {:.1}s elapsed",
                                i, block.header.nonce, hash_rate, elapsed
                            ),
                        );
                        last_log_nonce = block.header.nonce;
                    }
                }
            });
        }

        let mut result = None;
        while let Some(res) = set.join_next().await {
            if let Ok(Some(block)) = res {
                result = Some(block);
                break;
            }
        }
        set.abort_all();

        if let Some(block) = result {
            return Ok(block);
        }

        attempts += 1;
        utils::log_warning(
            utils::LogCategory::Core,
            &format!(
                "Failed to mine block: exhausted all nonce values (attempt {}/{}) Retrying with new timestamp...",
                attempts, CONFIG.max_mining_attempts
            ),
        );
    }
}
