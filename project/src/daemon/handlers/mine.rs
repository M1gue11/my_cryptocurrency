// Mining Handlers
use crate::daemon::types::{MineBlockResponse, RpcResponse};
use crate::model::get_node_mut;
use crate::utils::transaction_model_to_view;

pub async fn handle_mine_block(id: Option<u64>) -> RpcResponse {
    let mut node = get_node_mut().await;

    match node.mine() {
        Ok(block) => {
            // Extract block information before saving
            let block_hash = hex::encode(block.header_hash());
            let nonce = block.header.nonce;
            let transactions: Vec<_> = block
                .transactions
                .iter()
                .map(|tx| transaction_model_to_view(tx))
                .collect();

            node.save_node();
            let response = MineBlockResponse {
                success: true,
                block_hash: Some(block_hash),
                transactions,
                nonce: Some(nonce),
                error: None,
            };
            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
        Err(e) => {
            let response = MineBlockResponse {
                success: false,
                block_hash: None,
                transactions: Vec::new(),
                nonce: None,
                error: Some(e),
            };
            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
    }
}
