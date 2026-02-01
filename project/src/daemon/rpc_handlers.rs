use crate::common::rpc_types::RpcError;
use crate::daemon::state::DaemonState;
use crate::model::{get_node, get_node_mut};
use std::sync::Arc;

pub async fn dispatch_rpc_method(
    method: &str,
    params: serde_json::Value,
    daemon_state: Arc<DaemonState>,
) -> Result<serde_json::Value, RpcError> {
    match method {
        "daemon.ping" => handle_daemon_ping().await,
        "node.status" => handle_node_status().await,
        "chain.status" => handle_chain_status().await,
        "mine.block" => handle_mine_block().await,
        "wallet.balance" => handle_wallet_balance(params, daemon_state).await,
        _ => Err(RpcError::method_not_found()),
    }
}

async fn handle_daemon_ping() -> Result<serde_json::Value, RpcError> {
    Ok(serde_json::json!("pong"))
}

async fn handle_node_status() -> Result<serde_json::Value, RpcError> {
    let node = get_node().await;
    let state = node.get_node_state().await;

    serde_json::to_value(state).map_err(|e| RpcError::internal_error(format!("Serialization error: {}", e)))
}

async fn handle_chain_status() -> Result<serde_json::Value, RpcError> {
    let node = get_node().await;
    let block_count = node.blockchain.chain.len();
    let validation = node.validate_bc();

    let mut status = serde_json::json!({
        "blocks": block_count,
        "valid": validation.is_ok(),
    });

    if block_count > 0 {
        let last_block = node.blockchain.chain.last().unwrap();
        status["last_block_hash"] = serde_json::json!(hex::encode(last_block.header_hash()));
        status["last_block_date"] = serde_json::json!(last_block.header.timestamp.to_string());
    }

    Ok(status)
}

async fn handle_mine_block() -> Result<serde_json::Value, RpcError> {
    let mut node = get_node_mut().await;

    let block = node.mine().map_err(|e| RpcError::internal_error(format!("Mining failed: {}", e)))?;

    // Copy data from block before saving
    let hash = hex::encode(block.header_hash());
    let tx_count = block.transactions.len();
    let nonce = block.header.nonce;
    let timestamp = block.header.timestamp.to_string();

    node.save_node();

    let block_info = serde_json::json!({
        "hash": hash,
        "transactions": tx_count,
        "nonce": nonce,
        "timestamp": timestamp,
    });

    Ok(block_info)
}

async fn handle_wallet_balance(
    params: serde_json::Value,
    daemon_state: Arc<DaemonState>,
) -> Result<serde_json::Value, RpcError> {
    let wallet_name = params.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());

    let wallet = daemon_state
        .get_wallet(wallet_name)
        .await
        .map_err(|e| RpcError::internal_error(e))?;

    let utxos = wallet.get_wallet_utxos();
    let total: i64 = utxos.iter().map(|u| u.output.value).sum();

    let balance_info = serde_json::json!({
        "utxos": utxos.len(),
        "total_balance": total,
    });

    Ok(balance_info)
}
