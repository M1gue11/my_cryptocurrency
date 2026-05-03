// Node RPC Types
use serde::{Deserialize, Serialize};

use crate::daemon::types::TransactionViewResponse;

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeStatusResponse {
    pub version: String,
    pub peers_connected: usize,
    pub advertised_addr: String,
    pub block_height: usize,
    pub top_block_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MempoolEntry {
    pub tx: TransactionViewResponse,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MempoolResponse {
    pub count: usize,
    pub transactions: Vec<MempoolEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeInitResponse {
    pub success: bool,
    pub block_count: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimpleSuccessResponse {
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewPeerConnectionParams {
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewPeerConnectionResponse {
    pub success: bool,
    pub fail_message: Option<String>,
}
