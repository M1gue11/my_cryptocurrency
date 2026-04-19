// Mining Handlers
use crate::daemon::types::rpc::{INTERNAL_ERROR, INVALID_PARAMS};
use crate::daemon::types::{KeepMiningParams, MineBlockResponse, RpcResponse};
use crate::model::miner::{mine, submit_block};
use crate::model::{get_node, get_node_mut};
use crate::security_utils::bytes_to_hex_string;
use crate::utils::{format_difficulty, format_target_hex, transaction_model_to_view};

pub async fn handle_mine_block(id: Option<u64>) -> RpcResponse {
    let mined_block = match mine().await {
        Ok(b) => b,
        Err(e) => {
            let mut response = MineBlockResponse::empty();
            response.error = Some(e);
            return RpcResponse::success(id, serde_json::to_value(response).unwrap());
        }
    };

    let (block, next_target) = match submit_block(mined_block).await {
        Ok(result) => result,
        Err(e) => return RpcResponse::error(id, INTERNAL_ERROR, e),
    };

    let transactions: Vec<_> = block
        .transactions
        .iter()
        .map(|tx| transaction_model_to_view(tx))
        .collect();
    let response = MineBlockResponse {
        success: true,
        block_hash: Some(bytes_to_hex_string(&block.header_hash())),
        transactions,
        nonce: Some(block.header.nonce),
        error: None,
        target: Some(format_target_hex(block.header.target)),
        next_target: Some(format_target_hex(next_target)),
        next_difficulty: Some(format_difficulty(next_target)),
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
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

    let mut node = get_node_mut().await;
    node.set_keep_mining_flag(params.keep_mining);

    let response = serde_json::json!({
        "success": true,
    });
    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}
