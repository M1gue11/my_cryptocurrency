// Transaction Handlers
use crate::daemon::types::rpc::{INTERNAL_ERROR, INVALID_PARAMS};
use crate::daemon::types::{
    RpcResponse, TransactionViewParams, TransactionViewResponse, TxInputInfo, TxOutputInfo,
};
use crate::db::repository::LedgerRepository;

pub async fn handle_transaction_view(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: TransactionViewParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e));
        }
    };

    let tx_id_bytes = match hex::decode(&params.id) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut array = [0u8; 32];
            array.copy_from_slice(&bytes);
            array
        }
        _ => {
            return RpcResponse::error(
                id,
                INVALID_PARAMS,
                "Invalid transaction ID format".to_string(),
            );
        }
    };

    let repo = LedgerRepository::new();
    match repo.get_transaction(&tx_id_bytes) {
        Ok(tx) => {
            let inputs: Vec<TxInputInfo> = tx
                .inputs
                .iter()
                .map(|i| TxInputInfo {
                    prev_tx_id: hex::encode(i.prev_tx_id),
                    output_index: i.output_index,
                    public_key: i.public_key.clone(),
                    signature: i.signature.clone(),
                })
                .collect();

            let outputs: Vec<TxOutputInfo> = tx
                .outputs
                .iter()
                .map(|o| TxOutputInfo {
                    value: o.value,
                    address: o.address.clone(),
                })
                .collect();

            let response = TransactionViewResponse {
                id: hex::encode(tx.id()),
                date: tx.date.to_string(),
                message: tx.message.clone(),
                inputs,
                outputs,
                is_coinbase: tx.is_coinbase(),
                size: tx.size(),
            };

            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
        Err(_) => RpcResponse::error(
            id,
            INVALID_PARAMS,
            "Unknown transaction ID: transaction not found".to_string(),
        ),
    }
}
