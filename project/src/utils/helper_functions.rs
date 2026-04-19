use chrono::{NaiveDateTime, Utc};
use primitive_types::U256;

use crate::{
    daemon::types::{TransactionViewResponse, TxInputInfo, TxOutputInfo},
    globals::CONSENSUS_RULES,
    model::Transaction,
    security_utils::bytes_to_hex_string,
};

pub fn format_date(date: &NaiveDateTime) -> String {
    date.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn get_current_timestamp() -> NaiveDateTime {
    Utc::now().naive_utc()
}

pub fn assert_parent_dir_exists(file_path: &str) -> Result<(), String> {
    let path = std::path::Path::new(file_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directories: {}", e))?;
    }
    Ok(())
}

pub fn format_target_hex(target: U256) -> String {
    format!("0x{:064x}", target)
}

pub fn leading_zero_bits(target: U256) -> u32 {
    if target.is_zero() {
        256
    } else {
        (256 - target.bits()) as u32
    }
}

pub fn difficulty_ratio(target: U256) -> f64 {
    let initial = CONSENSUS_RULES.initial_target;
    let safe_target = target.max(U256::one());

    u256_to_f64(initial) / u256_to_f64(safe_target)
}

pub fn format_difficulty(target: U256) -> String {
    format!(
        "{} zero bits, {:.2}x",
        leading_zero_bits(target),
        difficulty_ratio(target)
    )
}

fn u256_to_f64(value: U256) -> f64 {
    let mut bytes = [0u8; 32];
    value.to_big_endian(&mut bytes);

    bytes
        .iter()
        .fold(0.0_f64, |acc, byte| acc * 256.0 + f64::from(*byte))
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
