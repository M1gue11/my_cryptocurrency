// Node RPC Types
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeStatusResponse {
    pub version: String,
    pub peers_connected: usize,
    pub block_height: usize,
    pub top_block_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MempoolEntry {
    pub tx_id: String,
    pub amount: i64,
    pub fee: i64,
    pub fee_per_byte: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MempoolResponse {
    pub count: usize,
    pub transactions: Vec<MempoolEntry>,
}
