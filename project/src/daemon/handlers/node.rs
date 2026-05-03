// Node Handlers
use crate::daemon::types::{
    INVALID_PARAMS, MempoolEntry, MempoolResponse, NewPeerConnectionParams,
    NewPeerConnectionResponse, NodeStatusResponse, RpcResponse,
};
use crate::model::{get_node, get_node_mut, node::restart_node};
use crate::network::server::connect_to_new_peer;
use crate::security_utils::bytes_to_hex_string;
use crate::utils::transaction_model_to_view;

pub async fn handle_node_status(id: Option<u64>) -> RpcResponse {
    let state = get_node().await.get_node_state().await;

    let response = NodeStatusResponse {
        version: state.version.version.to_string(),
        peers_connected: state.peers_connected,
        advertised_addr: state.version.advertised_addr,
        block_height: state.version.height as usize,
        top_block_hash: bytes_to_hex_string(&state.version.top_hash),
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

pub async fn handle_node_init(id: Option<u64>) -> RpcResponse {
    restart_node().await;
    let node = get_node().await;

    let block_count = node.blockchain.height();
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
            tx: transaction_model_to_view(&mtx.tx),
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

pub async fn handle_node_connect(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: NewPeerConnectionParams = match serde_json::from_value(params) {
        Ok(v) => v,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e)),
    };
    let fail_message = match connect_to_new_peer(params.address).await {
        Ok(_) => None,
        Err(e) => Some(e),
    };
    let response = NewPeerConnectionResponse {
        success: fail_message.is_none(),
        fail_message: fail_message,
    };
    RpcResponse::success(id, serde_json::json!(response))
}
