use once_cell::sync::Lazy;
use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    pub persisted_chain_path: String,
    pub db_path: String,
    pub miner_wallet_seed_path: String,
    pub miner_wallet_password: String,
    pub max_mining_attempts: u32,
    pub p2p_port: u16,
    pub peers: Vec<String>,
    pub rpc_port: u16,
    pub http_port: u16,
}

pub static CONFIG: Lazy<Settings> = Lazy::new(|| {
    dotenv::dotenv().ok();

    Settings {
        persisted_chain_path: env::var("PERSISTED_CHAIN_PATH")
            .unwrap_or_else(|_| "saved_files".to_string()),
        db_path: env::var("DB_PATH").unwrap_or_else(|_| "saved_files/bd".to_string()),
        miner_wallet_seed_path: env::var("MINER_WALLET_SEED_PATH")
            .unwrap_or_else(|_| "keys/miner_wallet.json".to_string()),
        miner_wallet_password: env::var("MINER_WALLET_PASSWORD")
            .unwrap_or_else(|_| "password123".to_string()),
        max_mining_attempts: env::var("MAX_MINING_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1_000_000),
        p2p_port: env::var("P2P_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6000),
        peers: env::var("PEERS")
            .unwrap_or_else(|_| "".to_string())
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),
        rpc_port: env::var("RPC_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7000),
        http_port: env::var("HTTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7001),
    }
});

pub struct ConsensusRules {
    /// Difficulty level for mining in number of leading zero bits
    pub difficulty: usize,
    pub max_block_size_kb: f32,
    pub block_reward: i64,
}

pub static CONSENSUS_RULES: Lazy<ConsensusRules> = Lazy::new(|| ConsensusRules {
    difficulty: 8,
    max_block_size_kb: 1.0,
    block_reward: 1 * COIN,
});

pub const COIN: i64 = 1_000_000;

// pub static NODE: Lazy<RwLock<Node>> = Lazy::new(|| RwLock::new(Node::new()));
