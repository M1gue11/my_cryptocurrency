use once_cell::sync::Lazy;
use primitive_types::U256;
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
    pub pbkdf2_iterations: u32,
    pub log_file_path: String,
    pub log_mode: String,
    pub mining_threads: usize,
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
            .unwrap_or(3),
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
        pbkdf2_iterations: env::var("PBKDF2_ITERATIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        log_file_path: env::var("LOG_FILE_PATH")
            .unwrap_or_else(|_| "saved_files/node.log".to_string()),
        log_mode: env::var("LOG_MODE").unwrap_or_else(|_| "full".to_string()),
        mining_threads: env::var("MINING_THREADS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
    }
});

pub struct ConsensusRules {
    /// Initial mining target as a 256-bit integer. Hash must be less than this value.
    pub initial_target: U256,
    pub max_block_size_kb: f32,
    /// TODO: Implement block reward halving every N blocks
    pub block_reward: i64,
    /// LWMA window size: how many recent blocks to consider for difficulty adjustment
    pub lwma_n: usize,
    /// Target block time in seconds
    pub target_block_time_secs: u64,
}

pub static CONSENSUS_RULES: Lazy<ConsensusRules> = Lazy::new(|| ConsensusRules {
    // 20 leading zero bits: hash < 2^(256-20) = U256::MAX >> 20
    initial_target: U256::MAX >> 20u32,
    max_block_size_kb: 1.0,
    block_reward: 1 * COIN,
    lwma_n: 10,
    target_block_time_secs: 20,
});

pub const COIN: i64 = 1_000_000;

// pub static NODE: Lazy<RwLock<Node>> = Lazy::new(|| RwLock::new(Node::new()));
