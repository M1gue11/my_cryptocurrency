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
    pub target: Option<String>,
    pub next_target: Option<String>,
    pub next_difficulty: Option<String>,
}
impl MineBlockResponse {
    pub fn empty() -> Self {
        MineBlockResponse {
            success: false,
            block_hash: None,
            transactions: Vec::new(),
            nonce: None,
            error: None,
            target: None,
            next_target: None,
            next_difficulty: None,
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct KeepMiningParams {
    pub keep_mining: bool,
}
