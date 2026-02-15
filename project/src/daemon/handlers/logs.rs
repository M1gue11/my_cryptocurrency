use crate::daemon::types::RpcResponse;
use crate::utils::{self, LogCategory, LogLevel};

pub async fn handle_get_logs(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let category = params
        .get("category")
        .and_then(|v| v.as_str())
        .and_then(parse_category);

    let level = params
        .get("level")
        .and_then(|v| v.as_str())
        .and_then(parse_level);

    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let logs = utils::get_logs(category, level, limit);
    RpcResponse::success(id, serde_json::to_value(logs).unwrap())
}

fn parse_category(s: &str) -> Option<LogCategory> {
    match s.to_lowercase().as_str() {
        "core" => Some(LogCategory::Core),
        "p2p" => Some(LogCategory::P2P),
        "rpc" => Some(LogCategory::RPC),
        _ => None,
    }
}

fn parse_level(s: &str) -> Option<LogLevel> {
    match s.to_lowercase().as_str() {
        "info" => Some(LogLevel::Info),
        "warning" | "warn" => Some(LogLevel::Warning),
        "error" => Some(LogLevel::Error),
        _ => None,
    }
}
