// Mining RPC Types
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct MineBlockResponse {
    pub success: bool,
    pub block_hash: Option<String>,
    pub transactions_count: Option<usize>,
    pub nonce: Option<u32>,
    pub error: Option<String>,
}
