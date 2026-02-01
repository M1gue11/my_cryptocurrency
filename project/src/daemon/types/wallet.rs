// Wallet RPC Types
use serde::{Deserialize, Serialize};

use crate::daemon::types::UtxoInfo;

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletNewParams {
    pub password: String,
    pub path: String,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletNewResponse {
    pub success: bool,
    pub address: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletInfo {
    pub name: String,
    pub balance: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletListResponse {
    pub wallets: Vec<WalletInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletAddressParams {
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletAddressResponse {
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletBalanceParams {
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletBalanceResponse {
    pub balance: i64,
    pub utxo_count: usize,
    pub utxos: Vec<UtxoInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletSendParams {
    pub from: Option<String>,
    pub to: String,
    pub amount: i64,
    pub fee: Option<i64>,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletSendResponse {
    pub success: bool,
    pub tx_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletGenerateKeysParams {
    pub count: Option<u32>,
    pub name: Option<String>,
    /// 0 = receive, 1 = change
    pub derivation_type: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeneratedKey {
    pub address: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletGenerateKeysResponse {
    pub keys: Vec<GeneratedKey>,
}
