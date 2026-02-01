use crate::model::Wallet;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DaemonState {
    pub loaded_wallets: Arc<RwLock<HashMap<String, Wallet>>>,
}

impl DaemonState {
    pub fn new() -> Self {
        Self {
            loaded_wallets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_wallet(&self, name: Option<String>) -> Result<Wallet, String> {
        let name = name.unwrap_or_else(|| "miner_wallet".to_string());
        let wallets = self.loaded_wallets.read().await;
        wallets
            .get(&name)
            .cloned()
            .ok_or_else(|| format!("Wallet '{}' not found", name))
    }

    pub async fn add_wallet(&self, name: String, wallet: Wallet) {
        let mut wallets = self.loaded_wallets.write().await;
        wallets.insert(name, wallet);
    }

    pub async fn list_wallets(&self) -> Vec<(String, Wallet)> {
        let wallets = self.loaded_wallets.read().await;
        wallets.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}
