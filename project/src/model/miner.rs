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
    pub wallet: Option<Wallet>,
}

impl Miner {
    pub fn new() -> Self {
        Miner {
            wallet: Self::try_load_wallet(),
        }
    }

    /// Try to load the miner keystore from disk. Logs and returns `None` on
    /// failure so the node can keep running and the wallet can be created
    /// later via RPC.
    fn try_load_wallet() -> Option<Wallet> {
        match Wallet::from_keystore_file(
            &CONFIG.miner_wallet_seed_path,
            &CONFIG.miner_wallet_password,
        ) {
            Ok(w) => Some(w),
            Err(e) => {
                utils::log_warning(
                    utils::LogCategory::Core,
                    &format!(
                        "Miner wallet not loaded yet (Path: {} - Error: {}). \
                         The node will keep running; create the wallet via \
                         'wallet new --path {}' to enable mining.",
                        CONFIG.miner_wallet_seed_path, e, CONFIG.miner_wallet_seed_path
                    ),
                );
                None
            }
        }
    }

    /// Return the loaded wallet, attempting a lazy reload from disk if it is
    /// not yet available. Used by mining flows so the operator can create the
    /// keystore after node start without restarting the process.
    pub fn ensure_wallet(&mut self) -> Result<&mut Wallet, String> {
        if self.wallet.is_none() {
            self.wallet = Self::try_load_wallet();
        }
        self.wallet.as_mut().ok_or_else(|| {
            format!(
                "Miner wallet not available at '{}'. Create it via 'wallet new --path {}' and try again.",
                CONFIG.miner_wallet_seed_path, CONFIG.miner_wallet_seed_path
            )
        })
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

fn sorted_legit_txs_by_fee_rate(mempool: &Vec<MempoolTx>) -> Vec<&MempoolTx> {
    let mut txs = get_legit_txs(mempool);
    txs.sort_by(|a, b| {
        let fee_rate_a = a.calculate_fee_per_byte();
        let fee_rate_b = b.calculate_fee_per_byte();
        fee_rate_b
            .partial_cmp(&fee_rate_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    txs
}

fn configured_max_block_size_bytes() -> usize {
    (CONSENSUS_RULES.max_block_size_kb * 1000.0) as usize
}

fn build_transactions_with_coinbase(
    selected_txs: &[Transaction],
    receive_addr: &str,
    total_fees: i64,
) -> Vec<Transaction> {
    let mut block_txs = selected_txs.to_vec();
    block_txs.insert(
        0,
        Transaction::new_coinbase(receive_addr.to_string(), total_fees),
    );
    block_txs
}

fn build_block_from_transactions(
    previous_hash: [u8; 32],
    target: U256,
    transactions: Vec<Transaction>,
) -> Block {
    let mut block = Block::new(previous_hash, target);
    block.transactions = transactions;
    block.evaluate_merkle_root();
    block
}

fn build_candidate_block(
    selected_txs: &[Transaction],
    candidate_tx: &Transaction,
    candidate_total_fees: i64,
    previous_hash: [u8; 32],
    target: U256,
    receive_addr: &str,
) -> Block {
    let mut candidate_txs = selected_txs.to_vec();
    candidate_txs.push(candidate_tx.clone());
    let block_txs =
        build_transactions_with_coinbase(&candidate_txs, receive_addr, candidate_total_fees);
    build_block_from_transactions(previous_hash, target, block_txs)
}

fn candidate_fits_block_size(candidate_block: &Block, max_block_size_bytes: usize) -> bool {
    candidate_block.size() <= max_block_size_bytes
}

fn select_transactions_for_block(
    txs: Vec<&MempoolTx>,
    previous_hash: [u8; 32],
    target: U256,
    receive_addr: &str,
    max_block_size_bytes: usize,
) -> (Vec<Transaction>, i64) {
    let mut selected_txs = Vec::new();
    let mut total_fees: i64 = 0;

    for mtx in txs {
        let candidate_total_fees = match total_fees.checked_add(mtx.calculate_fee()) {
            Some(fees) => fees,
            None => {
                utils::log_warning(
                    utils::LogCategory::Core,
                    "Skipping transaction because total fees would overflow.",
                );
                continue;
            }
        };

        let candidate_block = build_candidate_block(
            &selected_txs,
            &mtx.tx,
            candidate_total_fees,
            previous_hash,
            target,
            receive_addr,
        );

        if !candidate_fits_block_size(&candidate_block, max_block_size_bytes) {
            continue;
        }

        selected_txs.push(mtx.tx.clone());
        total_fees = candidate_total_fees;
    }

    (selected_txs, total_fees)
}

fn build_block(
    mempool: &Vec<MempoolTx>,
    previous_hash: [u8; 32],
    target: U256,
    receive_addr: &str,
) -> Block {
    let txs = sorted_legit_txs_by_fee_rate(mempool);
    let max_block_size_bytes = configured_max_block_size_bytes();
    let (selected_txs, total_fees) = select_transactions_for_block(
        txs,
        previous_hash,
        target,
        receive_addr,
        max_block_size_bytes,
    );
    let block_txs = build_transactions_with_coinbase(&selected_txs, receive_addr, total_fees);
    let block = build_block_from_transactions(previous_hash, target, block_txs);
    utils::log_info(
        utils::LogCategory::Core,
        &format!(
            "Current block size (bytes): {} / {}",
            block.size(),
            max_block_size_bytes
        ),
    );
    utils::log_info(
        utils::LogCategory::Core,
        &format!(
            "Transactions selected for block: {}",
            block.transactions.len()
        ),
    );
    block
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
        let (mempool, previous_hash, target, receive_addr) = node.prepare_mining_snapshot()?;
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
            match node.prepare_mining_snapshot() {
                Ok(snapshot) => snapshot,
                Err(e) => {
                    utils::log_warning(
                        utils::LogCategory::Core,
                        &format!(
                            "Keep mining cannot start a new round: {}. Disabling keep mining.",
                            e
                        ),
                    );
                    node.set_keep_mining_flag(false);
                    break;
                }
            }
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
