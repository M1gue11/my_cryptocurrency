// RPC client implementation (to be implemented in Phase 3)

use crate::common::rpc_types::{RpcRequest, RpcResponse};
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct RpcClient {
    client: Client,
    endpoint: String,
    next_id: AtomicU64,
}

impl RpcClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            endpoint: "http://127.0.0.1:7777/rpc".to_string(),
            next_id: AtomicU64::new(1),
        }
    }

    pub async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, String> {
        let req = RpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: self.next_id.fetch_add(1, Ordering::SeqCst),
        };

        let resp = self
            .client
            .post(&self.endpoint)
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?
            .json::<RpcResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(error) = resp.error {
            return Err(format!("RPC Error {}: {}", error.code, error.message));
        }

        serde_json::from_value(resp.result.unwrap())
            .map_err(|e| format!("Failed to deserialize result: {}", e))
    }

    pub async fn ping(&self) -> Result<String, String> {
        self.call("daemon.ping", serde_json::json!({})).await
    }
}
