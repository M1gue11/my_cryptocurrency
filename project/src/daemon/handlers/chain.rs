// Chain Handlers
use crate::daemon::types::rpc::INTERNAL_ERROR;
use crate::daemon::types::{
    BlockInfo, ChainShowResponse, ChainStatusResponse, RpcResponse, UtxoInfo, UtxosParams,
    UtxosResponse,
};
use crate::db::repository::LedgerRepository;
use crate::model::get_node;
use crate::utils::transaction_model_to_view;

pub async fn handle_chain_status(id: Option<u64>) -> RpcResponse {
    let node = get_node().await;
    let block_count = node.blockchain.chain.len();
    let validation = node.validate_bc();

    let (last_hash, last_date) = if block_count > 0 {
        let last_block = node.blockchain.chain.last().unwrap();
        (
            Some(hex::encode(last_block.header_hash())),
            Some(last_block.header.timestamp.to_string()),
        )
    } else {
        (None, None)
    };

    let response = ChainStatusResponse {
        block_count,
        is_valid: validation.is_ok(),
        last_block_hash: last_hash,
        last_block_date: last_date,
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

pub async fn handle_chain_show(id: Option<u64>) -> RpcResponse {
    let node = get_node().await;

    let blocks: Vec<BlockInfo> = node
        .blockchain
        .chain
        .iter()
        .enumerate()
        .map(|(i, block)| BlockInfo {
            height: i,
            hash: hex::encode(block.header_hash()),
            prev_hash: hex::encode(block.header.prev_block_hash),
            merkle_root: hex::encode(block.header.merkle_root),
            nonce: block.header.nonce,
            timestamp: block.header.timestamp.to_string(),
            transactions: block
                .transactions
                .iter()
                .map(|tx| transaction_model_to_view(tx))
                .collect(),
            size_bytes: block.size(),
        })
        .collect();

    let response = ChainShowResponse { blocks };
    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

pub async fn handle_chain_validate(id: Option<u64>) -> RpcResponse {
    let node = get_node().await;
    let validation = node.validate_bc();

    let response = match validation {
        Ok(is_valid) => serde_json::json!({
            "valid": is_valid,
            "error": null
        }),
        Err(e) => serde_json::json!({
            "valid": false,
            "error": e
        }),
    };

    RpcResponse::success(id, response)
}

pub async fn handle_node_save(id: Option<u64>) -> RpcResponse {
    let node = get_node().await;
    node.save_node();

    RpcResponse::success(id, serde_json::json!({ "success": true }))
}

pub async fn handle_chain_utxos(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: UtxosParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(_) => UtxosParams { limit: 20 },
    };

    let repo = LedgerRepository::new();
    let utxos = match repo.get_all_utxos(Some(params.limit as usize)) {
        Ok(u) => u,
        Err(e) => {
            return RpcResponse::error(id, INTERNAL_ERROR, format!("Failed to get UTXOs: {}", e));
        }
    };

    let utxo_list: Vec<UtxoInfo> = utxos
        .iter()
        .map(|u| UtxoInfo {
            tx_id: hex::encode(u.tx_id),
            index: u.index,
            value: u.output.value,
            address: u.output.address.clone(),
        })
        .collect();

    let total: i64 = utxo_list.iter().map(|u| u.value).sum();

    let response = UtxosResponse {
        utxos: utxo_list,
        total_value: total,
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}
