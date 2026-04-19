// Mining RPC Types
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::daemon::types::TransactionViewResponse;

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MiningInfoResponse {
    pub keep_mining_enabled: bool,
    pub is_currently_mining: bool,
    pub started_at: Option<NaiveDateTime>,
    pub last_mined_block: Option<MineBlockResponse>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeepMiningParams {
    pub keep_mining: bool,
}
