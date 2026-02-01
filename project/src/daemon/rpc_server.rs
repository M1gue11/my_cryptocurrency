// JSON-RPC server for the daemon
use crate::daemon::handlers::chain::{
    handle_chain_show, handle_chain_status, handle_chain_utxos, handle_chain_validate,
    handle_node_save,
};
use crate::daemon::handlers::mine::handle_mine_block;
use crate::daemon::handlers::node::{
    handle_node_clear_mempool, handle_node_init, handle_node_mempool, handle_node_status,
};
use crate::daemon::handlers::tx::handle_transaction_view;
use crate::daemon::handlers::wallet::{
    handle_wallet_address, handle_wallet_balance, handle_wallet_generate_keys, handle_wallet_new,
    handle_wallet_send,
};
use crate::daemon::types::rpc::{INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR};
use crate::daemon::types::{RpcRequest, RpcResponse};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// daemon RPC server
pub struct RpcServer {
    port: u16,
}

impl RpcServer {
    pub async fn new(port: u16) -> Self {
        RpcServer { port }
    }

    /// Inicia o servidor RPC
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;

        println!("[RPC] Server listening on {}", addr);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            println!("[RPC] New connection from {}", peer_addr);

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream).await {
                    eprintln!("[RPC] Connection error: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            // Connection closed
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = process_request(trimmed).await;
        let response_json = serde_json::to_string(&response)? + "\n";
        writer.write_all(response_json.as_bytes()).await?;
    }

    Ok(())
}

pub async fn process_request(request_str: &str) -> RpcResponse {
    // Parse JSON-RPC request
    let request: RpcRequest = match serde_json::from_str(request_str) {
        Ok(req) => req,
        Err(e) => {
            return RpcResponse::error(None, PARSE_ERROR, format!("Parse error: {}", e));
        }
    };
    println!("[RPC] Received request: {}", request.method);

    // Validate jsonrpc version
    if request.jsonrpc != "2.0" {
        return RpcResponse::error(
            request.id,
            INVALID_REQUEST,
            "Invalid JSON-RPC version".to_string(),
        );
    }

    // Route to appropriate handler
    match request.method.as_str() {
        // Node methods
        "node_status" => handle_node_status(request.id).await,
        "node_init" => handle_node_init(request.id).await,
        "node_mempool" => handle_node_mempool(request.id).await,
        "node_clear_mempool" => handle_node_clear_mempool(request.id).await,
        "node_save" => handle_node_save(request.id).await,

        // Mining methods
        "mine_block" => handle_mine_block(request.id).await,

        // Chain methods
        "chain_status" => handle_chain_status(request.id).await,
        "chain_show" => handle_chain_show(request.id).await,
        "chain_validate" => handle_chain_validate(request.id).await,
        "chain_utxos" => handle_chain_utxos(request.id, request.params).await,

        // Wallet methods
        "wallet_new" => handle_wallet_new(request.id, request.params).await,
        "wallet_address" => handle_wallet_address(request.id, request.params).await,
        "wallet_balance" => handle_wallet_balance(request.id, request.params).await,
        "wallet_send" => handle_wallet_send(request.id, request.params).await,
        "wallet_generate_keys" => handle_wallet_generate_keys(request.id, request.params).await,

        // Transaction methods
        "transaction_view" => handle_transaction_view(request.id, request.params).await,

        _ => RpcResponse::error(
            request.id,
            METHOD_NOT_FOUND,
            format!("Method not found: {}", request.method),
        ),
    }
}
