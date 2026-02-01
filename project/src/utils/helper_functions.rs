use chrono::NaiveDateTime;

use crate::{
    daemon::types::{TransactionViewResponse, TxInputInfo, TxOutputInfo},
    model::Transaction,
    security_utils::bytes_to_hex_string,
};

pub fn format_date(date: &NaiveDateTime) -> String {
    date.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn assert_parent_dir_exists(file_path: &str) -> Result<(), String> {
    let path = std::path::Path::new(file_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directories: {}", e))?;
    }
    Ok(())
}

pub fn transaction_model_to_view(tx: &Transaction) -> TransactionViewResponse {
    TransactionViewResponse {
        id: bytes_to_hex_string(&tx.id()),
        date: tx.date.to_string(),
        message: tx.message.clone(),
        inputs: tx
            .inputs
            .iter()
            .map(|input| TxInputInfo {
                prev_tx_id: bytes_to_hex_string(&input.prev_tx_id),
                output_index: input.output_index,
                signature: input.signature.clone(),
                public_key: input.public_key.clone(),
            })
            .collect(),
        outputs: tx
            .outputs
            .iter()
            .map(|output| TxOutputInfo {
                value: output.value,
                address: output.address.clone(),
            })
            .collect(),
        is_coinbase: tx.is_coinbase(),
        size: tx.size(),
    }
}
