// Node Handlers
use crate::daemon::types::{MempoolEntry, MempoolResponse, NodeStatusResponse, RpcResponse};
use crate::model::{get_node, get_node_mut, node::restart_node};
use crate::security_utils::bytes_to_hex_string;

pub async fn handle_node_status(id: Option<u64>) -> RpcResponse {
    let state = get_node().await.get_node_state().await;

    let response = NodeStatusResponse {
        version: state.version.version.to_string(),
        peers_connected: state.peers_connected,
        block_height: state.version.height as usize,
        top_block_hash: bytes_to_hex_string(&state.version.top_hash),
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

pub async fn handle_node_init(id: Option<u64>) -> RpcResponse {
    restart_node().await;
    let node = get_node().await;

    let block_count = node.blockchain.chain.len();
    let response = serde_json::json!({
        "success": true,
        "block_count": block_count
    });

    RpcResponse::success(id, response)
}

pub async fn handle_node_mempool(id: Option<u64>) -> RpcResponse {
    let node = get_node().await;

    let transactions: Vec<MempoolEntry> = node
        .get_mempool()
        .iter()
        .map(|mtx| MempoolEntry {
            tx_id: bytes_to_hex_string(&mtx.tx.id()),
            amount: mtx.tx.amount(),
            fee: mtx.calculate_fee(),
            fee_per_byte: mtx.calculate_fee_per_byte(),
        })
        .collect();

    let response = MempoolResponse {
        count: transactions.len(),
        transactions,
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

pub async fn handle_node_clear_mempool(id: Option<u64>) -> RpcResponse {
    let mut node = get_node_mut().await;
    node.clear_mempool();
    node.save_node();

    RpcResponse::success(id, serde_json::json!({ "success": true }))
}
