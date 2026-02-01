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
    pub signature: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxOutputInfo {
    pub value: i64,
    pub address: String,
}

#[derive(Serialize, Deserialize)]
pub struct TransactionViewResponse {
    pub id: String,
    pub date: String,
    pub message: Option<String>,
    pub inputs: Vec<TxInputInfo>,
    pub outputs: Vec<TxOutputInfo>,
    pub is_coinbase: bool,
    pub size: usize,
}
impl std::fmt::Debug for TransactionViewResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransactionView")
            .field("id", &self.id)
            .field("inputs", &self.inputs)
            .field("outputs", &self.outputs)
            .field("date", &self.date)
            .field("message", &self.message)
            .field("is coinbase", &self.is_coinbase)
            .field("size in bytes", &self.size)
            .finish()
    }
}
