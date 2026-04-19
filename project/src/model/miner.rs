use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tokio::task::JoinSet;

use primitive_types::U256;

use crate::model::get_node_mut;
use crate::utils::log_info;
use crate::{
    daemon::types::MineBlockResponse,
    globals::{CONFIG, CONSENSUS_RULES},
    model::{Block, MempoolTx, Transaction, Wallet, transaction::TxId},
    security_utils::{bytes_to_hex_string, hash_meets_target},
    utils::{self, format_difficulty, format_target_hex, transaction_model_to_view},
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

async fn mine_block_impl(
    mempool: Vec<MempoolTx>,
    previous_hash: [u8; 32],
    target: U256,
    receive_addr: String,
    cancel: Arc<AtomicBool>,
) -> Result<Block, String> {
    let threads = CONFIG.mining_threads.max(1);
    let mut attempts = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Mining cancelled".to_string());
        }

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
            let cancel = Arc::clone(&cancel);
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
                    if cancel.load(Ordering::Relaxed) {
                        return None;
                    }
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

        if cancel.load(Ordering::Relaxed) {
            return Err(
                "Mining cancelled because the chain tip changed or mining was interrupted."
                    .to_string(),
            );
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

pub async fn mine() -> Result<Block, String> {
    let (mempool, previous_hash, target, receive_addr, cancel) = {
        let mut node = get_node_mut().await;
        if node.is_keep_mining_enabled() || node.is_mining_task_running() {
            return Err(
                "Auto-mining is active. Disable keep mining before mining manually.".to_string(),
            );
        }
        let cancel = node.mining_cancel_flag();
        node.reset_mining_cancel();
        let (mempool, previous_hash, target, receive_addr) = node.prepare_mining_snapshot();
        (mempool, previous_hash, target, receive_addr, cancel)
    };
    mine_block_impl(mempool, previous_hash, target, receive_addr, cancel).await
}

pub async fn submit_block(mined_block: Block) -> Result<(Block, U256), String> {
    let mut node = get_node_mut().await;

    let block = node.submit_mined_block(mined_block)?;
    let next_target = node.blockchain.calculate_next_target();
    let mined_block_response = build_mined_block_response(&block, next_target);
    node.set_last_mined_block(mined_block_response);
    node.save_node();

    Ok((block, next_target))
}

pub fn build_mined_block_response(block: &Block, next_target: U256) -> MineBlockResponse {
    let transactions = block
        .transactions
        .iter()
        .map(transaction_model_to_view)
        .collect();

    MineBlockResponse {
        success: true,
        block_hash: Some(bytes_to_hex_string(&block.header_hash())),
        transactions,
        nonce: Some(block.header.nonce),
        error: None,
        target: Some(format_target_hex(block.header.target)),
        next_target: Some(format_target_hex(next_target)),
        next_difficulty: Some(format_difficulty(next_target)),
    }
}

pub async fn run_keep_mining_loop(keep_mining_enabled: Arc<AtomicBool>, cancel: Arc<AtomicBool>) {
    loop {
        if !keep_mining_enabled.load(Ordering::Relaxed) {
            break;
        }

        let (mempool, previous_hash, target, receive_addr) = {
            let mut node = get_node_mut().await;
            if !node.is_keep_mining_enabled() {
                break;
            }
            node.reset_mining_cancel();
            node.prepare_mining_snapshot()
        };

        let mined_block = mine_block_impl(
            mempool,
            previous_hash,
            target,
            receive_addr,
            Arc::clone(&cancel),
        )
        .await;

        let block = match mined_block {
            Ok(block) => block,
            Err(e) => {
                if e.starts_with("Mining cancelled") {
                    utils::log_info(
                        utils::LogCategory::Core,
                        &format!("Keep mining round cancelled: {}", e),
                    );
                } else {
                    utils::log_warning(
                        utils::LogCategory::Core,
                        &format!("Keep mining round failed: {}", e),
                    );
                }
                continue;
            }
        };

        match submit_block(block).await {
            Ok((submitted_block, _)) => {
                log_info(
                    utils::LogCategory::Core,
                    &format!(
                        "Keep mining mined block {} successfully.",
                        bytes_to_hex_string(&submitted_block.id())
                    ),
                );
            }
            Err(e) => {
                utils::log_warning(
                    utils::LogCategory::Core,
                    &format!("Keep mining submit failed, retrying with fresh work: {}", e),
                );
            }
        }
    }

    let mut node = get_node_mut().await;
    node.finish_background_task();
    utils::log_info(utils::LogCategory::Core, "Keep mining loop stopped.");
}
