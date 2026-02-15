// Wallet Handlers
use crate::daemon::types::rpc::INVALID_PARAMS;
use crate::daemon::types::{
    GeneratedKey, RpcResponse, UtxoInfo, WalletAddressParams, WalletAddressResponse,
    WalletBalanceParams, WalletBalanceResponse, WalletGenerateKeysParams,
    WalletGenerateKeysResponse, WalletImportParams, WalletNewParams, WalletNewResponse,
    WalletSendParams, WalletSendResponse,
};
use crate::model::wallet::DerivationType;
use crate::model::{TxOutput, Wallet, get_node_mut};
use crate::security_utils::Keystore;
use crate::security_utils::bytes_to_hex_string;

pub async fn handle_import_wallet(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: WalletImportParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e));
        }
    };

    let is_imported_wallet = true;

    // Check if keystore file exists
    let keystore_exists = std::path::Path::new(&params.path).exists();
    if !keystore_exists {
        return RpcResponse::error(
            id,
            INVALID_PARAMS,
            format!("KeyStore file not found at path: {}", params.path),
        );
    }

    let mut wallet = match Wallet::from_keystore_file(&params.path, &params.password) {
        Ok(w) => w,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("Failed to load wallet: {}", e));
        }
    };

    let address = wallet.get_receive_addr();
    let response = WalletNewResponse {
        success: true,
        address: Some(address),
        is_imported_wallet,
    };
    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

pub async fn handle_new_wallet(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: WalletNewParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e));
        }
    };

    // This is SUPER important to avoid overwriting existing wallets
    if std::path::Path::new(&params.path).exists() {
        return RpcResponse::error(
            id,
            INVALID_PARAMS,
            format!(
                "Keystore file already exists at path! Use a different path to avoid overwriting!"
            ),
        );
    }

    let mut wallet = match Keystore::new_seed(&params.password, &params.path) {
        Ok(seed) => Wallet::from_seed(seed),
        Err(create_err) => {
            return RpcResponse::error(
                id,
                INVALID_PARAMS,
                format!("Failed to create new wallet: {}", create_err),
            );
        }
    };
    let response = WalletNewResponse {
        success: true,
        address: Some(wallet.get_receive_addr()),
        is_imported_wallet: false,
    };
    return RpcResponse::success(id, serde_json::to_value(response).unwrap());
}

pub async fn handle_wallet_address(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: WalletAddressParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(_) => {
            return RpcResponse::error(id, INVALID_PARAMS, "Invalid params".to_string());
        }
    };

    let mut wallet = match Wallet::from_keystore_file(&params.key_path, &params.password) {
        Ok(w) => w,
        Err(_) => {
            return RpcResponse::error(id, INVALID_PARAMS, "Wallet not found".to_string());
        }
    };

    let address = wallet.get_receive_addr();
    let response = WalletAddressResponse { address };

    RpcResponse::success(id, serde_json::to_value(response).unwrap())
}

pub async fn handle_wallet_balance(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: WalletBalanceParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(_) => {
            return RpcResponse::error(id, INVALID_PARAMS, "Invalid params".to_string());
        }
    };

    let wallet = match Wallet::from_keystore_file(&params.key_path, &params.password) {
        Ok(w) => w,
        Err(_) => {
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

pub async fn handle_wallet_send(id: Option<u64>, params: serde_json::Value) -> RpcResponse {
    let params: WalletSendParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e));
        }
    };

    let mut wallet = match Wallet::from_keystore_file(&params.from.key_path, &params.from.password)
    {
        Ok(w) => w,
        Err(_) => {
            return RpcResponse::error(id, INVALID_PARAMS, "Wallet not found".to_string());
        }
    };

    let outputs = vec![TxOutput {
        value: params.amount,
        address: params.to,
    }];

    match wallet.send_tx(outputs, params.fee, params.message) {
        Ok(mempool_tx) => {
            let mut node = get_node_mut().await;
            let tx_id = mempool_tx.tx.id();
            match node.receive_transaction(mempool_tx) {
                Ok(_) => {
                    node.persist_mempool();
                    let response = WalletSendResponse {
                        success: true,
                        tx_id: Some(bytes_to_hex_string(&tx_id)),
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

pub async fn handle_wallet_generate_keys(
    id: Option<u64>,
    params: serde_json::Value,
) -> RpcResponse {
    let params: WalletGenerateKeysParams = match serde_json::from_value(params) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("Invalid params: {}", e));
        }
    };

    let count = params.count.unwrap_or(5);
    let derivation_type = match params.derivation_type {
        None => None,
        Some(0) => Some(DerivationType::Receive),
        Some(1) => Some(DerivationType::Change),
        Some(_) => {
            return RpcResponse::error(
                id,
                INVALID_PARAMS,
                "Invalid derivation_type: expected 0 (Receive) or 1 (Change)".to_string(),
            );
        }
    };

    let wallet = match Wallet::from_keystore_file(&params.wallet.key_path, &params.wallet.password)
    {
        Ok(w) => w,
        Err(_) => {
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
