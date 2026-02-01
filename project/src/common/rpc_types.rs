use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: u64,
}

impl RpcResponse {
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: u64, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcError {
    pub fn method_not_found() -> Self {
        Self {
            code: -32601,
            message: "Method not found".to_string(),
        }
    }

    pub fn internal_error(msg: String) -> Self {
        Self {
            code: -32603,
            message: msg,
        }
    }

    pub fn invalid_params(msg: String) -> Self {
        Self {
            code: -32602,
            message: msg,
        }
    }

    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
        }
    }
}
