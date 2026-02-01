// Servidor JSON-RPC para o daemon
//
// Escuta conexões TCP locais e processa requisições JSON-RPC 2.0.
// Cada método corresponde a uma funcionalidade do node.

use crate::daemon::types::rpc::{
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
};
use crate::daemon::types::{
    BlockInfo, ChainShowResponse, ChainStatusResponse, GeneratedKey, MempoolEntry, MempoolResponse,
    MineBlockResponse, NodeStatusResponse, RpcRequest, RpcResponse, TransactionViewParams,
    TransactionViewResponse, TxInputInfo, TxOutputInfo, UtxoInfo, UtxosParams, UtxosResponse,
    WalletAddressParams, WalletAddressResponse, WalletBalanceParams, WalletBalanceResponse,
    WalletGenerateKeysParams, WalletGenerateKeysResponse, WalletInfo, WalletListResponse,
    WalletNewParams, WalletNewResponse, WalletSendParams, WalletSendResponse,
};
use crate::db::repository::LedgerRepository;
use crate::model::wallet::DerivationType;
use crate::model::{TxOutput, Wallet, get_node, get_node_mut, node::restart_node};
use crate::security_utils::bytes_to_hex_string;

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

/// Porta padrão do RPC server
pub const DEFAULT_RPC_PORT: u16 = 7000;

/// Servidor RPC do daemon
pub struct RpcServer {
    port: u16,
    /// Wallets carregadas na sessão (nome -> Wallet)
    wallets: Arc<RwLock<Vec<(String, Wallet)>>>,
}

impl RpcServer {
    pub async fn new(port: u16) -> Self {
        // Carrega a wallet do minerador por padrão
        let miner_wallet = {
            let node = get_node().await;
            node.miner.wallet.clone()
        };

        let wallets = Arc::new(RwLock::new(vec![(
            "miner_wallet".to_string(),
            miner_wallet,
        )]));

        RpcServer { port, wallets }
    }

    /// Inicia o servidor RPC
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;

        println!("[RPC] Server listening on {}", addr);

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            println!("[RPC] New connection from {}", peer_addr);

            let wallets = self.wallets.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, wallets).await {
                    eprintln!("[RPC] Connection error: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    wallets: Arc<RwLock<Vec<(String, Wallet)>>>,
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

        let response = process_request(trimmed, wallets.clone()).await;
        let response_json = serde_json::to_string(&response)? + "\n";
        writer.write_all(response_json.as_bytes()).await?;
    }

    Ok(())
}

async fn process_request(
    request_str: &str,
    wallets: Arc<RwLock<Vec<(String, Wallet)>>>,
) -> RpcResponse {
    // Parse JSON-RPC request
    let request: RpcRequest = match serde_json::from_str(request_str) {
        Ok(req) => req,
        Err(e) => {
            return RpcResponse::error(None, PARSE_ERROR, format!("Parse error: {}", e));
        }
    };

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

        // Mining methods
        "mine_block" => handle_mine_block(request.id).await,

        // Chain methods
        "chain_status" => handle_chain_status(request.id).await,
        "chain_show" => handle_chain_show(request.id).await,
        "chain_validate" => handle_chain_validate(request.id).await,
        "chain_save" => handle_chain_save(request.id).await,
        "chain_utxos" => handle_chain_utxos(request.id, request.params).await,

        // Wallet methods
        "wallet_new" => handle_wallet_new(request.id, request.params, wallets).await,
        "wallet_list" => handle_wallet_list(request.id, wallets).await,
        "wallet_address" => handle_wallet_address(request.id, request.params, wallets).await,
        "wallet_balance" => handle_wallet_balance(request.id, request.params, wallets).await,
        "wallet_send" => handle_wallet_send(request.id, request.params, wallets).await,
        "wallet_generate_keys" => {
            handle_wallet_generate_keys(request.id, request.params, wallets).await
        }

        // Transaction methods
        "transaction_view" => handle_transaction_view(request.id, request.params).await,

        _ => RpcResponse::error(
            request.id,
            METHOD_NOT_FOUND,
            format!("Method not found: {}", request.method),
        ),
    }
}

// ============================================================================
// Node Handlers
// ============================================================================

async fn handle_node_status(id: Option<u64>) -> RpcResponse {
    let state = get_node().await.get_node_state().await;

    let response = NodeStatusResponse {
        version: state.version.version.to_string(),
        peers_connected: state.peers_connected,
        block_height: state.version.height as usize,
        top_block_hash: bytes_to_hex_string(&state.version.top_hash),
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

async fn handle_node_init(id: Option<u64>) -> RpcResponse {
    restart_node().await;
    let node = get_node().await;

    let block_count = node.blockchain.chain.len();
    let response = serde_json::json!({
        "success": true,
        "block_count": block_count
    });

    RpcResponse::success(id, response)
}

async fn handle_node_mempool(id: Option<u64>) -> RpcResponse {
    let node = get_node().await;

    let transactions: Vec<MempoolEntry> = node
        .get_mempool()
        .iter()
        .map(|mtx| MempoolEntry {
            tx_id: bytes_to_hex_string(&mtx.tx.id()),
            amount: mtx.tx.amount(),
            fee: mtx.calculate_fee(),
            fee_per_byte: mtx.calculate_fee_per_byte(),
        })
        .collect();

    let response = MempoolResponse {
        count: transactions.len(),
        transactions,
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

async fn handle_node_clear_mempool(id: Option<u64>) -> RpcResponse {
    let mut node = get_node_mut().await;
    node.clear_mempool();
    node.save_node();

    RpcResponse::success(id, serde_json::json!({ "success": true }))
}

// ============================================================================
// Mining Handlers
// ============================================================================

async fn handle_mine_block(id: Option<u64>) -> RpcResponse {
    let mut node = get_node_mut().await;

    match node.mine() {
        Ok(block) => {
            // Extrai informações do bloco antes de salvar
            let block_hash = hex::encode(block.header_hash());
            let tx_count = block.transactions.len();
            let nonce = block.header.nonce;

            node.save_node();

            let response = MineBlockResponse {
                success: true,
                block_hash: Some(block_hash),
                transactions_count: Some(tx_count),
                nonce: Some(nonce),
                error: None,
            };
            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
        Err(e) => {
            let response = MineBlockResponse {
                success: false,
                block_hash: None,
                transactions_count: None,
                nonce: None,
                error: Some(e),
            };
            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
    }
}

// ============================================================================
// Chain Handlers
// ============================================================================

async fn handle_chain_status(id: Option<u64>) -> RpcResponse {
    let node = get_node().await;
    let block_count = node.blockchain.chain.len();
    let validation = node.validate_bc();

    let (last_hash, last_date) = if block_count > 0 {
        let last_block = node.blockchain.chain.last().unwrap();
        (
            Some(hex::encode(last_block.header_hash())),
            Some(last_block.header.timestamp.to_string()),
        )
    } else {
        (None, None)
    };

    let response = ChainStatusResponse {
        block_count,
        is_valid: validation.is_ok(),
        last_block_hash: last_hash,
        last_block_date: last_date,
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

async fn handle_chain_show(id: Option<u64>) -> RpcResponse {
    let node = get_node().await;

    let blocks: Vec<BlockInfo> = node
        .blockchain
        .chain
        .iter()
        .enumerate()
        .map(|(i, block)| BlockInfo {
            height: i,
            hash: hex::encode(block.header_hash()),
            prev_hash: hex::encode(block.header.prev_block_hash),
            merkle_root: hex::encode(block.header.merkle_root),
            nonce: block.header.nonce,
            timestamp: block.header.timestamp.to_string(),
            transactions_count: block.transactions.len(),
            size_bytes: block.size(),
        })
        .collect();

    let response = ChainShowResponse { blocks };
    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

async fn handle_chain_validate(id: Option<u64>) -> RpcResponse {
    let node = get_node().await;
    let validation = node.validate_bc();

    let response = match validation {
        Ok(is_valid) => serde_json::json!({
            "valid": is_valid,
            "error": null
        }),
        Err(e) => serde_json::json!({
            "valid": false,
            "error": e
        }),
    };

    RpcResponse::success(id, response)
}

async fn handle_chain_save(id: Option<u64>) -> RpcResponse {
    let node = get_node().await;
    node.save_node();

    RpcResponse::success(id, serde_json::json!({ "success": true }))
}

async fn handle_chain_utxos(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: UtxosParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(_) => UtxosParams { limit: 20 },
    };

    let repo = LedgerRepository::new();
    let utxos = match repo.get_all_utxos(Some(params.limit as usize)) {
        Ok(u) => u,
        Err(e) => {
            return RpcResponse::error(id, INTERNAL_ERROR, format!("Failed to get UTXOs: {}", e));
        }
    };

    let utxo_list: Vec<UtxoInfo> = utxos
        .iter()
        .map(|u| UtxoInfo {
            tx_id: hex::encode(u.tx_id),
            index: u.index,
            value: u.output.value,
            address: u.output.address.clone(),
        })
        .collect();

    let total: i64 = utxo_list.iter().map(|u| u.value).sum();

    let response = UtxosResponse {
        utxos: utxo_list,
        total_value: total,
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

// ============================================================================
// Wallet Handlers
// ============================================================================

fn resolve_wallet<'a>(
    name: Option<String>,
    wallets: &'a mut Vec<(String, Wallet)>,
) -> Option<&'a mut Wallet> {
    let name = name.unwrap_or_else(|| "miner_wallet".to_string());
    for (loaded_name, wallet) in wallets.iter_mut() {
        if *loaded_name == name {
            return Some(wallet);
        }
    }
    None
}

async fn handle_wallet_new(
    id: Option<u64>,
    params: serde_json::Value,
    wallets: Arc<RwLock<Vec<(String, Wallet)>>>,
) -> RpcResponse {
    let params: WalletNewParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e));
        }
    };

    let mut wallet = match Wallet::from_keystore_file(&params.path, &params.password) {
        Ok(w) => w,
        Err(_) => Wallet::new(&params.password, &params.path),
    };

    let address = wallet.get_receive_addr();

    if let Some(name) = params.name {
        let mut wallets = wallets.write().await;
        wallets.push((name, wallet));
    }

    let response = WalletNewResponse {
        success: true,
        address: Some(address),
        error: None,
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

async fn handle_wallet_list(
    id: Option<u64>,
    wallets: Arc<RwLock<Vec<(String, Wallet)>>>,
) -> RpcResponse {
    let wallets = wallets.read().await;

    let wallet_infos: Vec<WalletInfo> = wallets
        .iter()
        .map(|(name, w)| WalletInfo {
            name: name.clone(),
            balance: w.calculate_balance(),
        })
        .collect();

    let response = WalletListResponse {
        wallets: wallet_infos,
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

async fn handle_wallet_address(
    id: Option<u64>,
    params: serde_json::Value,
    wallets: Arc<RwLock<Vec<(String, Wallet)>>>,
) -> RpcResponse {
    let params: WalletAddressParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(_) => WalletAddressParams { name: None },
    };

    let mut wallets = wallets.write().await;
    let wallet = match resolve_wallet(params.name, &mut wallets) {
        Some(w) => w,
        None => {
            return RpcResponse::error(id, INVALID_PARAMS, "Wallet not found".to_string());
        }
    };

    let address = wallet.get_receive_addr();
    let response = WalletAddressResponse { address };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

async fn handle_wallet_balance(
    id: Option<u64>,
    params: serde_json::Value,
    wallets: Arc<RwLock<Vec<(String, Wallet)>>>,
) -> RpcResponse {
    let params: WalletBalanceParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(_) => WalletBalanceParams { name: None },
    };

    let mut wallets = wallets.write().await;
    let wallet = match resolve_wallet(params.name, &mut wallets) {
        Some(w) => w,
        None => {
            return RpcResponse::error(id, INVALID_PARAMS, "Wallet not found".to_string());
        }
    };

    let utxos = wallet.get_wallet_utxos();
    let total: i64 = utxos.iter().map(|u| u.output.value).sum();

    let utxo_infos: Vec<UtxoInfo> = utxos
        .iter()
        .map(|u| UtxoInfo {
            tx_id: hex::encode(u.tx_id),
            index: u.index,
            value: u.output.value,
            address: u.output.address.clone(),
        })
        .collect();

    let response = WalletBalanceResponse {
        balance: total,
        utxo_count: utxo_infos.len(),
        utxos: utxo_infos,
    };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

async fn handle_wallet_send(
    id: Option<u64>,
    params: serde_json::Value,
    wallets: Arc<RwLock<Vec<(String, Wallet)>>>,
) -> RpcResponse {
    let params: WalletSendParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e));
        }
    };

    let mut wallets = wallets.write().await;
    let wallet = match resolve_wallet(params.from, &mut wallets) {
        Some(w) => w,
        None => {
            return RpcResponse::error(id, INVALID_PARAMS, "Wallet not found".to_string());
        }
    };

    let outputs = vec![TxOutput {
        value: params.amount,
        address: params.to,
    }];

    match wallet.send_tx(outputs, params.fee, params.message) {
        Ok(mempool_tx) => {
            let tx_id = hex::encode(mempool_tx.tx.id());
            let mut node = get_node_mut().await;

            match node.receive_transaction(mempool_tx) {
                Ok(_) => {
                    node.persist_mempool();
                    let response = WalletSendResponse {
                        success: true,
                        tx_id: Some(tx_id),
                        error: None,
                    };
                    RpcResponse::success(id, serde_json::to_value(response).unwrap())
                }
                Err(e) => {
                    let response = WalletSendResponse {
                        success: false,
                        tx_id: None,
                        error: Some(e.to_string()),
                    };
                    RpcResponse::success(id, serde_json::to_value(response).unwrap())
                }
            }
        }
        Err(e) => {
            let response = WalletSendResponse {
                success: false,
                tx_id: None,
                error: Some(e.to_string()),
            };
            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
    }
}

async fn handle_wallet_generate_keys(
    id: Option<u64>,
    params: serde_json::Value,
    wallets: Arc<RwLock<Vec<(String, Wallet)>>>,
) -> RpcResponse {
    let params: WalletGenerateKeysParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(_) => WalletGenerateKeysParams {
            count: Some(5),
            name: None,
            derivation_type: None,
        },
    };

    let count = params.count.unwrap_or(5);
    let derivation_type = params.derivation_type.map(|t| {
        if t == 0 {
            DerivationType::Receive
        } else {
            DerivationType::Change
        }
    });

    let mut wallets = wallets.write().await;
    let wallet = match resolve_wallet(params.name, &mut wallets) {
        Some(w) => w,
        None => {
            return RpcResponse::error(id, INVALID_PARAMS, "Wallet not found".to_string());
        }
    };

    let keys = wallet.generate_n_keys(count, None, derivation_type);

    let generated: Vec<GeneratedKey> = keys
        .iter()
        .map(|k| GeneratedKey {
            address: k.get_address(),
            public_key: hex::encode(k.get_public_key().as_bytes()),
        })
        .collect();

    let response = WalletGenerateKeysResponse { keys: generated };
    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

// ============================================================================
// Transaction Handlers
// ============================================================================

async fn handle_transaction_view(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: TransactionViewParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e));
        }
    };

    let tx_id_bytes = match hex::decode(&params.id) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut array = [0u8; 32];
            array.copy_from_slice(&bytes);
            array
        }
        _ => {
            return RpcResponse::error(
                id,
                INVALID_PARAMS,
                "Invalid transaction ID format".to_string(),
            );
        }
    };

    let repo = LedgerRepository::new();
    match repo.get_transaction(&tx_id_bytes) {
        Ok(tx) => {
            let inputs: Vec<TxInputInfo> = tx
                .inputs
                .iter()
                .map(|i| TxInputInfo {
                    prev_tx_id: hex::encode(i.prev_tx_id),
                    output_index: i.output_index,
                    public_key: i.public_key.clone(),
                })
                .collect();

            let outputs: Vec<TxOutputInfo> = tx
                .outputs
                .iter()
                .map(|o| TxOutputInfo {
                    value: o.value,
                    address: o.address.clone(),
                })
                .collect();

            let response = TransactionViewResponse {
                id: hex::encode(tx.id()),
                date: tx.date.to_string(),
                message: tx.message.clone(),
                inputs,
                outputs,
                is_coinbase: tx.is_coinbase(),
            };

            RpcResponse::success(id, serde_json::to_value(response).unwrap())
        }
        Err(_) => RpcResponse::error(id, INTERNAL_ERROR, "Transaction not found".to_string()),
    }
}
