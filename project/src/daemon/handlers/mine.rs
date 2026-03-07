// Mining Handlers
use crate::daemon::types::{MineBlockResponse, RpcResponse};
use crate::model::get_node_mut;
use crate::security_utils::bytes_to_hex_string;
use crate::utils::transaction_model_to_view;

pub async fn handle_mine_block(id: Option<u64>) -> RpcResponse {
    let mut node = get_node_mut().await;

    match node.mine() {
        Ok(block) => {
            // Extract block information before saving
            let block_hash = bytes_to_hex_string(&block.header_hash());
            let nonce = block.header.nonce;
            let transactions: Vec<_> = block
                .transactions
                .iter()
                .map(|tx| transaction_model_to_view(tx))
                .collect();
            let response = MineBlockResponse {
                success: true,
                block_hash: Some(block_hash),
                transactions,
                nonce: Some(nonce),
                error: None,
                difficulty: Some(block.header.difficulty),
                next_difficulty: Some(node.blockchain.calculate_next_difficulty()),
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
                difficulty: None,
                next_difficulty: None,
            };
            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
    }
}
