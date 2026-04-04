// Mining Handlers
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::daemon::types::rpc::INVALID_PARAMS;
use crate::daemon::types::{KeepMiningParams, MineBlockResponse, RpcResponse};
use crate::model::miner::mine_block;
use crate::model::{get_node, get_node_mut};
use crate::security_utils::bytes_to_hex_string;
use crate::utils::{self, transaction_model_to_view};

async fn do_mine_and_submit(cancel: Arc<AtomicBool>) -> Result<MineBlockResponse, String> {
    let (mempool, previous_hash, difficulty, receive_addr) = {
        let mut node = get_node_mut().await;
        node.prepare_mining()
    };

    let mined_block = mine_block(mempool, previous_hash, difficulty, receive_addr, cancel).await?;

    let mut node = get_node_mut().await;
    match node.submit_mined_block(mined_block) {
        Ok(block) => {
            let block_hash = bytes_to_hex_string(&block.header_hash());
            let nonce = block.header.nonce;
            let difficulty = block.header.difficulty;
            let transactions: Vec<_> = block
                .transactions
                .iter()
                .map(|tx| transaction_model_to_view(tx))
                .collect();
            let next_difficulty = node.blockchain.calculate_next_difficulty();
            node.save_node();
            Ok(MineBlockResponse {
                success: true,
                block_hash: Some(block_hash),
                transactions,
                nonce: Some(nonce),
                error: None,
                difficulty: Some(difficulty),
                next_difficulty: Some(next_difficulty),
            })
        }
        Err(e) => Err(e),
    }
}

pub async fn handle_mine_block(id: Option<u64>) -> RpcResponse {
    let cancel = Arc::new(AtomicBool::new(false));
    match do_mine_and_submit(cancel).await {
        Ok(response) => RpcResponse::success(id, serde_json::to_value(response).unwrap()),
        Err(e) => {
            let response = MineBlockResponse {
                success: false,
                block_hash: None,
                transactions: Vec::new(),
                nonce: None,
                error: Some(e),
                difficulty: None,
                next_difficulty: None,
            };
            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
    }
}

pub async fn handle_get_mining_info(id: Option<u64>) -> RpcResponse {
    let mining_info = get_node().await.get_mining_info();
    let response = serde_json::to_value(mining_info).unwrap();
    RpcResponse::success(id, response)
}

pub async fn handle_keep_mining(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: KeepMiningParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e)),
    };

    {
        let mut node = get_node_mut().await;
        node.set_keep_mining_flag(params.keep_mining);
    }

    if params.keep_mining {
        let cancel = {
            let node = get_node().await;
            node.get_keep_mining_arc()
        };
        tokio::spawn(async move {
            utils::log_info(
                utils::LogCategory::Core,
                "Continuous mining started.",
            );
            loop {
                {
                    let node = get_node().await;
                    if !node.get_keep_mining_flag() {
                        break;
                    }
                }
                match do_mine_and_submit(Arc::clone(&cancel)).await {
                    Ok(resp) => utils::log_info(
                        utils::LogCategory::Core,
                        &format!(
                            "Continuous mining: block mined successfully. hash={:?}",
                            resp.block_hash
                        ),
                    ),
                    Err(e) if e == "Mining cancelled" => break,
                    Err(e) => {
                        utils::log_error(
                            utils::LogCategory::Core,
                            &format!("Continuous mining error: {}", e),
                        );
                        break;
                    }
                }
            }
            utils::log_info(
                utils::LogCategory::Core,
                "Continuous mining stopped.",
            );
        });
    }

    RpcResponse::success(id, serde_json::json!({"success": true}))
}
