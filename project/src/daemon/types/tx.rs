// Transaction RPC Types
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionViewParams {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxInputInfo {
    pub prev_tx_id: String,
    pub output_index: usize,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxOutputInfo {
    pub value: i64,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionViewResponse {
    pub id: String,
    pub date: String,
    pub message: Option<String>,
    pub inputs: Vec<TxInputInfo>,
    pub outputs: Vec<TxOutputInfo>,
    pub is_coinbase: bool,
}
