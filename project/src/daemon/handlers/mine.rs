// Mining Handlers
use crate::daemon::types::rpc::INVALID_PARAMS;
use crate::daemon::types::{KeepMiningParams, MineBlockResponse, RpcResponse};
use crate::model::miner::mine_block;
use crate::model::{get_node, get_node_mut};
use crate::security_utils::bytes_to_hex_string;
use crate::utils::{format_difficulty, format_target_hex, transaction_model_to_view};

pub async fn handle_mine_block(id: Option<u64>) -> RpcResponse {
    let (mempool, previous_hash, target, receive_addr) = {
        let mut node = get_node_mut().await;
        node.prepare_mining()
    };

    let mined_block = match mine_block(mempool, previous_hash, target, receive_addr).await {
        Ok(b) => b,
        Err(e) => {
            let response = MineBlockResponse {
                success: false,
                block_hash: None,
                transactions: Vec::new(),
                nonce: None,
                error: Some(e),
                target: None,
                difficulty: None,
                next_target: None,
                next_difficulty: None,
            };
            return RpcResponse::success(id, serde_json::to_value(response).unwrap());
        }
    };
    // submit the mined block to the node
    let mut node = get_node_mut().await;
    match node.submit_mined_block(mined_block) {
        Ok(block) => {
            let (block_hash, nonce, transactions, target) = {
                let block_hash = bytes_to_hex_string(&block.header_hash());
                let nonce = block.header.nonce;
                let transactions: Vec<_> = block
                    .transactions
                    .iter()
                    .map(|tx| transaction_model_to_view(tx))
                    .collect();
                let target = block.header.target;

                (block_hash, nonce, transactions, target)
            };
            let next_target = node.blockchain.calculate_next_target();
            let response = MineBlockResponse {
                success: true,
                block_hash: Some(block_hash),
                transactions,
                nonce: Some(nonce),
                error: None,
                target: Some(format_target_hex(target)),
                difficulty: Some(format_difficulty(target)),
                next_target: Some(format_target_hex(next_target)),
                next_difficulty: Some(format_difficulty(next_target)),
            };
            node.save_node();
            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
        Err(e) => {
            let response = MineBlockResponse {
                success: false,
                block_hash: None,
                transactions: Vec::new(),
                nonce: None,
                error: Some(e),
                target: None,
                difficulty: None,
                next_target: None,
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

    let mut node = get_node_mut().await;
    node.set_keep_mining_flag(params.keep_mining);

    let response = serde_json::json!({
        "success": true,
    });
    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}
