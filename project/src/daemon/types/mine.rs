// Mining RPC Types
use serde::{Deserialize, Serialize};

use crate::daemon::types::TransactionViewResponse;

#[derive(Serialize, Deserialize, Debug)]
pub struct MineBlockResponse {
    pub success: bool,
    pub transactions: Vec<TransactionViewResponse>,
    pub block_hash: Option<String>,
    pub nonce: Option<u32>,
    pub error: Option<String>,
}
