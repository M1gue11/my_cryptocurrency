use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::daemon::types::{
    ChainShowResponse, ChainStatusResponse, MempoolResponse, MineBlockResponse, NodeStatusResponse,
    RpcRequest, RpcResponse, TransactionViewResponse, UtxosResponse, WalletAddressResponse,
    WalletBalanceResponse, WalletGenerateKeysResponse, WalletListResponse, WalletNewResponse,
    WalletSendResponse,
};

/// Contador global de IDs de requisição
static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Cliente RPC para comunicação com o daemon
pub struct RpcClient {
    host: String,
    port: u16,
}

impl RpcClient {
    pub fn new(host: &str, port: u16) -> Self {
        RpcClient {
            host: host.to_string(),
            port,
        }
    }

    /// Conecta ao daemon e retorna se a conexão foi bem-sucedida
    pub async fn ping(&self) -> bool {
        self.call::<serde_json::Value>("node_status", serde_json::json!({}))
            .await
            .is_ok()
    }

    /// Faz uma chamada RPC genérica
    pub async fn call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, String> {
        let addr = format!("{}:{}", self.host, self.port);

        let stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("Failed to connect to daemon: {}", e))?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(REQUEST_ID.fetch_add(1, Ordering::SeqCst)),
        };

        let request_json = serde_json::to_string(&request).unwrap() + "\n";
        writer
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let response: RpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = response.error {
            return Err(format!("RPC error: {}", error.message));
        }

        let result = response.result.ok_or("Empty response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse result: {}", e))
    }

    // ========================================================================
    // Node Methods
    // ========================================================================

    pub async fn node_status(&self) -> Result<NodeStatusResponse, String> {
        self.call("node_status", serde_json::json!({})).await
    }

    pub async fn node_init(&self) -> Result<serde_json::Value, String> {
        self.call("node_init", serde_json::json!({})).await
    }

    pub async fn node_mempool(&self) -> Result<MempoolResponse, String> {
        self.call("node_mempool", serde_json::json!({})).await
    }

    pub async fn node_clear_mempool(&self) -> Result<serde_json::Value, String> {
        self.call("node_clear_mempool", serde_json::json!({})).await
    }

    // ========================================================================
    // Mining Methods
    // ========================================================================

    pub async fn mine_block(&self) -> Result<MineBlockResponse, String> {
        self.call("mine_block", serde_json::json!({})).await
    }

    // ========================================================================
    // Chain Methods
    // ========================================================================

    pub async fn chain_status(&self) -> Result<ChainStatusResponse, String> {
        self.call("chain_status", serde_json::json!({})).await
    }

    pub async fn chain_show(&self) -> Result<ChainShowResponse, String> {
        self.call("chain_show", serde_json::json!({})).await
    }

    pub async fn chain_validate(&self) -> Result<serde_json::Value, String> {
        self.call("chain_validate", serde_json::json!({})).await
    }

    pub async fn chain_save(&self) -> Result<serde_json::Value, String> {
        self.call("chain_save", serde_json::json!({})).await
    }

    pub async fn chain_utxos(&self, limit: u32) -> Result<UtxosResponse, String> {
        self.call("chain_utxos", serde_json::json!({ "limit": limit }))
            .await
    }

    // ========================================================================
    // Wallet Methods
    // ========================================================================

    pub async fn wallet_new(
        &self,
        password: &str,
        path: &str,
        name: Option<&str>,
    ) -> Result<WalletNewResponse, String> {
        self.call(
            "wallet_new",
            serde_json::json!({
                "password": password,
                "path": path,
                "name": name
            }),
        )
        .await
    }

    pub async fn wallet_list(&self) -> Result<WalletListResponse, String> {
        self.call("wallet_list", serde_json::json!({})).await
    }

    pub async fn wallet_address(
        &self,
        name: Option<&str>,
    ) -> Result<WalletAddressResponse, String> {
        self.call("wallet_address", serde_json::json!({ "name": name }))
            .await
    }

    pub async fn wallet_balance(
        &self,
        name: Option<&str>,
    ) -> Result<WalletBalanceResponse, String> {
        self.call("wallet_balance", serde_json::json!({ "name": name }))
            .await
    }

    pub async fn wallet_send(
        &self,
        from: Option<&str>,
        to: &str,
        amount: i64,
        fee: Option<i64>,
        message: Option<&str>,
    ) -> Result<WalletSendResponse, String> {
        self.call(
            "wallet_send",
            serde_json::json!({
                "from": from,
                "to": to,
                "amount": amount,
                "fee": fee,
                "message": message
            }),
        )
        .await
    }

    pub async fn wallet_generate_keys(
        &self,
        count: u32,
        name: Option<&str>,
        derivation_type: Option<u32>,
    ) -> Result<WalletGenerateKeysResponse, String> {
        self.call(
            "wallet_generate_keys",
            serde_json::json!({
                "count": count,
                "name": name,
                "derivation_type": derivation_type
            }),
        )
        .await
    }

    // ========================================================================
    // Transaction Methods
    // ========================================================================

    pub async fn transaction_view(&self, id: &str) -> Result<TransactionViewResponse, String> {
        self.call("transaction_view", serde_json::json!({ "id": id }))
            .await
    }
}
