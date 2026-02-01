// Chain RPC Types
use serde::{Deserialize, Serialize};

use crate::daemon::types::TransactionViewResponse;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChainStatusResponse {
    pub block_count: usize,
    pub is_valid: bool,
    pub last_block_hash: Option<String>,
    pub last_block_date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockInfo {
    pub height: usize,
    pub hash: String,
    pub prev_hash: String,
    pub merkle_root: String,
    pub nonce: u32,
    pub timestamp: String,
    pub transactions: Vec<TransactionViewResponse>,
    pub size_bytes: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChainShowResponse {
    pub blocks: Vec<BlockInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UtxoInfo {
    pub tx_id: String,
    pub index: usize,
    pub value: i64,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UtxosParams {
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    20
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UtxosResponse {
    pub utxos: Vec<UtxoInfo>,
    pub total_value: i64,
}
