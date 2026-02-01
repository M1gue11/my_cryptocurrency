use crate::common::rpc_types::{RpcRequest, RpcResponse};
use crate::daemon::rpc_handlers::dispatch_rpc_method;
use crate::daemon::state::DaemonState;
use crate::globals::CONFIG;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use std::net::SocketAddr;
use std::sync::Arc;

pub struct AppState {
    pub daemon_state: Arc<DaemonState>,
}

pub async fn run_rpc_server() -> Result<(), String> {
    let daemon_state = Arc::new(DaemonState::new());

    // Initialize with miner wallet
    let miner_wallet = {
        use crate::model::get_node;
        let node = get_node().await;
        node.miner.wallet.clone()
    };
    daemon_state
        .add_wallet("miner_wallet".to_string(), miner_wallet)
        .await;

    let app_state = Arc::new(AppState {
        daemon_state: daemon_state.clone(),
    });

    let app = Router::new()
        .route("/rpc", post(handle_rpc_request))
        .with_state(app_state);

    let host = CONFIG.rpc_host.clone();
    let port = CONFIG.rpc_port;
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    println!("RPC Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind RPC server: {}", e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("RPC server error: {}", e))?;

    Ok(())
}

async fn handle_rpc_request(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RpcRequest>,
) -> Response {
    let id = req.id;

    match dispatch_rpc_method(&req.method, req.params, state.daemon_state.clone()).await {
        Ok(result) => Json(RpcResponse::success(id, result)).into_response(),
        Err(e) => Json(RpcResponse::error(id, e)).into_response(),
    }
}
