// Mining Handlers
use crate::daemon::types::{MineBlockResponse, RpcResponse};
use crate::model::get_node_mut;

pub async fn handle_mine_block(id: Option<u64>) -> RpcResponse {
    let mut node = get_node_mut().await;

    match node.mine() {
        Ok(block) => {
            // Extrai informações do bloco antes de salvar
            let block_hash = hex::encode(block.header_hash());
            let tx_count = block.transactions.len();
            let nonce = block.header.nonce;

            node.save_node();

            let response = MineBlockResponse {
                success: true,
                block_hash: Some(block_hash),
                transactions_count: Some(tx_count),
                nonce: Some(nonce),
                error: None,
            };
            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
        Err(e) => {
            let response = MineBlockResponse {
                success: false,
                block_hash: None,
                transactions_count: None,
                nonce: None,
                error: Some(e),
            };
            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
    }
}
