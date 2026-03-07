// Mining Handlers
use crate::daemon::types::{MineBlockResponse, RpcResponse};
use crate::model::get_node_mut;
use crate::model::miner::mine_block;
use crate::security_utils::bytes_to_hex_string;
use crate::utils::transaction_model_to_view;

pub async fn handle_mine_block(id: Option<u64>) -> RpcResponse {
    let (mempool, previous_hash, difficulty, receive_addr) = {
        let mut node = get_node_mut().await;
        node.prepare_mining()
    };

    let mined_block = match mine_block(mempool, previous_hash, difficulty, receive_addr).await {
        Ok(b) => b,
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
            return RpcResponse::success(id, serde_json::to_value(response).unwrap());
        }
    };
    // submit the mined block to the node
    let mut node = get_node_mut().await;
    match node.submit_mined_block(mined_block) {
        Ok(block) => {
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
